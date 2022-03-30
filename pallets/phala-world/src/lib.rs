#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	ensure,
	traits::{Currency, UnixTime},
	transactional, BoundedVec,
};
use frame_system::ensure_signed;

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::{sr25519, H256};
use sp_io::crypto::sr25519_verify;
use sp_runtime::{
	traits::{One, StaticLookup},
	DispatchResult,
};
use sp_std::prelude::*;

pub use pallet_rmrk_core::types::*;
pub use pallet_rmrk_market;

use rmrk_traits::{
	career::CareerType, egg::EggType, primitives::*, race::RaceType, status_type::StatusType,
	EggInfo, PreorderInfo,
};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub use pallet::*;

// #[cfg(feature = "std")]
// use serde::{Deserialize, Serialize};
//
// #[cfg_attr(feature = "std", derive(Serialize, Deserialize, PartialEq, Eq))]
// #[derive(Encode, Decode, RuntimeDebug, TypeInfo, Clone)]
// pub struct OverlordInfo<AccountId> {
// 	pub admin: AccountId,
// 	pub collection_id: u32,
// }

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
		sp_runtime::traits::Zero,
		traits::{ExistenceRequirement, ReservableCurrency},
	};
	use frame_system::{pallet_prelude::*, Origin};

	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	type PreorderInfoOf<T> = PreorderInfo<
		<T as frame_system::Config>::AccountId,
		BoundedVec<u8, <T as pallet_uniques::Config>::StringLimit>,
	>;
	//type OverlordInfoOf<T> = OverlordInfo<<T as frame_system::Config>::AccountId>;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_rmrk_core::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The origin which may forcibly buy, sell, list/unlist, offer & withdraw offer on Tokens
		type OverlordOrigin: EnsureOrigin<Self::Origin>;
		/// The market currency mechanism.
		type Currency: ReservableCurrency<Self::AccountId>;
		/// Time in UnixTime
		type Time: UnixTime;
		/// Seconds per Era that will increment the Era storage value every interval
		#[pallet::constant]
		type SecondsPerEra: Get<u64>;
		/// Price of Founder Egg Price
		#[pallet::constant]
		type FounderEggPrice: Get<BalanceOf<Self>>;
		/// Price of Legendary Egg Price
		#[pallet::constant]
		type LegendaryEggPrice: Get<BalanceOf<Self>>;
		/// Price of Normal Egg Price
		#[pallet::constant]
		type NormalEggPrice: Get<BalanceOf<Self>>;
		/// Max mint per Race
		#[pallet::constant]
		type MaxMintPerRace: Get<u32>;
		/// Max mint per Career
		#[pallet::constant]
		type MaxMintPerCareer: Get<u32>;
		/// Amount of food per Era
		#[pallet::constant]
		type FoodPerEra: Get<u8>;
		/// Max food an Egg can be fed per day
		#[pallet::constant]
		type MaxFoodFedPerEra: Get<u16>;
		/// Max food to feed your own Egg
		#[pallet::constant]
		type MaxFoodFeedSelf: Get<u8>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Stores all of the valid claimed spirits from the airdrop by serial id & bool true if claimed
	#[pallet::storage]
	#[pallet::getter(fn claimed_spirits)]
	pub type ClaimedSpirits<T: Config> = StorageMap<_, Twox64Concat, SerialId, bool>;

	/// Stores all of the valid claimed Eggs from the whitelist or preorder
	#[pallet::storage]
	#[pallet::getter(fn claimed_eggs)]
	pub type ClaimedEggs<T: Config> = StorageMap<_, Twox64Concat, SerialId, bool>;

	/// Preorder index that is the key to the Preorders StorageMap
	#[pallet::storage]
	#[pallet::getter(fn preorder_index)]
	pub type PreorderIndex<T: Config> = StorageValue<_, PreorderId, ValueQuery>;

	/// Preorder info map for user preorders
	#[pallet::storage]
	#[pallet::getter(fn preorders)]
	pub type Preorders<T: Config> = StorageMap<_, Twox64Concat, PreorderId, PreorderInfoOf<T>>;

	/// Stores all the Eggs and the information about the Egg pertaining to Hatch times and feeding
	#[pallet::storage]
	#[pallet::getter(fn eggs)]
	pub type Eggs<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, CollectionId, Blake2_128Concat, NftId, EggInfo>;

	/// Food per Owner where an owner gets 5 food per era
	#[pallet::storage]
	#[pallet::getter(fn get_food_by_owner)]
	pub type FoodByOwner<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, u8>;

	/// Phala World Zero Day `BlockNumber` this will be used to determine Eras
	#[pallet::storage]
	#[pallet::getter(fn zero_day)]
	pub(super) type ZeroDay<T: Config> = StorageValue<_, u64>;

	/// The current Era from the initial ZeroDay BlockNumber
	#[pallet::storage]
	#[pallet::getter(fn era)]
	pub type Era<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Spirits can be claimed
	#[pallet::storage]
	#[pallet::getter(fn can_claim_spirits)]
	pub type CanClaimSpirits<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Rare Eggs can be purchased
	#[pallet::storage]
	#[pallet::getter(fn can_purchase_rare_eggs)]
	pub type CanPurchaseRareEggs<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Eggs can be preordered
	#[pallet::storage]
	#[pallet::getter(fn can_preorder_eggs)]
	pub type CanPreorderEggs<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Race Type count
	#[pallet::storage]
	#[pallet::getter(fn race_type_count)]
	pub type RaceTypeLeft<T: Config> = StorageMap<_, Twox64Concat, RaceType, u32, ValueQuery>;

	/// Spirit Collection ID
	#[pallet::storage]
	#[pallet::getter(fn spirit_collection_id)]
	pub type SpiritCollectionId<T: Config> = StorageValue<_, CollectionId, OptionQuery>;

	/// Egg Collection ID
	#[pallet::storage]
	#[pallet::getter(fn egg_collection_id)]
	pub type EggCollectionId<T: Config> = StorageValue<_, CollectionId, OptionQuery>;

	/// Race StorageMap count
	#[pallet::storage]
	#[pallet::getter(fn career_type_count)]
	pub type CareerTypeLeft<T: Config> = StorageMap<_, Twox64Concat, CareerType, u32, ValueQuery>;

	/// Overlord Admin account of Phala World
	#[pallet::storage]
	#[pallet::getter(fn overlord)]
	pub(super) type Overlord<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(n: T::BlockNumber) {
			if let Some(zero_day) = <ZeroDay<T>>::get() {
				let current_time = T::Time::now().as_secs();
				if current_time > zero_day {
					let secs_since_zero_day = current_time - zero_day;
					let current_era = <Era<T>>::get();
					if secs_since_zero_day / T::SecondsPerEra::get() > current_era {
						let new_era = Era::<T>::mutate(|era| {
							*era = *era + 1;
							*era
						});
						Self::deposit_event(Event::NewEra { time: current_time, era: new_era });
					}
				}
			}
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		/// `BlockNumber` of Phala World Zero Day
		pub zero_day: Option<u64>,
		/// Overlord Admin account of Phala World
		pub overlord: Option<T::AccountId>,
		/// Current Era of Phala World
		pub era: u64,
		/// bool for if a Spirit is claimable
		pub can_claim_spirits: bool,
		/// bool for if a Rare Egg can be purchased
		pub can_purchase_rare_eggs: bool,
		/// bool for if an Egg can be preordered
		pub can_preorder_eggs: bool,
		/// CollectionId of Spirit Collection
		pub spirit_collection_id: Option<CollectionId>,
		/// CollectionId of Egg Collection
		pub egg_collection_id: Option<CollectionId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				zero_day: None,
				overlord: None,
				era: 0,
				can_claim_spirits: false,
				can_purchase_rare_eggs: false,
				can_preorder_eggs: false,
				spirit_collection_id: None,
				egg_collection_id: None,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			if let Some(ref zero_day) = self.zero_day {
				<ZeroDay<T>>::put(zero_day);
			}
			if let Some(ref overlord) = self.overlord {
				<Overlord<T>>::put(overlord);
			}
			let era = self.era;
			<Era<T>>::put(era);
			let can_claim_spirits = self.can_claim_spirits;
			<CanClaimSpirits<T>>::put(can_claim_spirits);
			let can_purchase_rare_eggs = self.can_purchase_rare_eggs;
			<CanPurchaseRareEggs<T>>::put(can_purchase_rare_eggs);
			let can_preorder_eggs = self.can_preorder_eggs;
			<CanPreorderEggs<T>>::put(can_preorder_eggs);
			if let Some(spirit_collection_id) = self.spirit_collection_id {
				<SpiritCollectionId<T>>::put(spirit_collection_id);
			}
			if let Some(egg_collection_id) = self.egg_collection_id {
				<EggCollectionId<T>>::put(egg_collection_id);
			}
			// Set max mints per race and career
			RaceTypeLeft::<T>::insert(RaceType::Cyborg, T::MaxMintPerRace::get());
			RaceTypeLeft::<T>::insert(RaceType::Pandroid, T::MaxMintPerRace::get());
			RaceTypeLeft::<T>::insert(RaceType::AISpectre, T::MaxMintPerRace::get());
			RaceTypeLeft::<T>::insert(RaceType::XGene, T::MaxMintPerRace::get());
			CareerTypeLeft::<T>::insert(CareerType::HardwareDruid, T::MaxMintPerCareer::get());
			CareerTypeLeft::<T>::insert(CareerType::HackerWizard, T::MaxMintPerCareer::get());
			CareerTypeLeft::<T>::insert(CareerType::RoboWarrior, T::MaxMintPerCareer::get());
			CareerTypeLeft::<T>::insert(CareerType::TradeNegotiator, T::MaxMintPerCareer::get());
			CareerTypeLeft::<T>::insert(CareerType::Web3Monk, T::MaxMintPerCareer::get());
		}
	}

	// Pallets use events to inform users when important changes are made.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Phala World clock zero day started
		WorldClockStarted {
			start_time: u64,
		},
		/// Start of a new era
		NewEra {
			time: u64,
			era: u64,
		},
		/// Spirit has been claimed from the whitelist
		SpiritClaimed {
			serial_id: SerialId,
			owner: T::AccountId,
		},
		/// Rare egg has been purchased
		RareEggPurchased {
			collection_id: CollectionId,
			nft_id: NftId,
			owner: T::AccountId,
		},
		/// A chance to get an egg through preorder
		EggPreordered {
			owner: T::AccountId,
			preorder_id: PreorderId,
		},
		/// Egg minted from the preorder
		EggMinted {
			collection_id: CollectionId,
			nft_id: NftId,
			owner: T::AccountId,
		},
		/// Spirit collection id was set
		SpiritCollectionIdSet {
			collection_id: CollectionId,
		},
		/// Egg collection id was set
		EggCollectionIdSet {
			collection_id: CollectionId,
		},
		/// Egg received food from an account
		EggFoodReceived {
			collection_id: CollectionId,
			nft_id: NftId,
			sender: T::AccountId,
			owner: T::AccountId,
		},
		/// Egg owner has initiated the hatching sequence
		StartedHatching {
			collection_id: CollectionId,
			nft_id: NftId,
			owner: T::AccountId,
		},
		/// A top 10 fed egg of the era has updated their hatch time
		HatchTimeUpdated {
			collection_id: CollectionId,
			nft_id: NftId,
			owner: T::AccountId,
			hatch_time: T::BlockNumber,
		},
		/// An egg has been hatched
		EggHatched {
			collection_id: CollectionId,
			nft_id: NftId,
			owner: T::AccountId,
		},
		/// Shell has been awakened from an egg being hatched and burned
		ShellAwakened {
			collection_id: CollectionId,
			nft_id: NftId,
			owner: T::AccountId,
			career: u8,
			race: u8,
		},
		/// Egg hatching has been disabled & no other eggs can be hatched
		EggHatchingDisabled {
			collection_id: CollectionId,
			can_hatch: bool,
		},
		/// Spirit Claims status has changed
		ClaimSpiritStatusChanged {
			status: bool,
		},
		/// Purchase Rare Eggs status has changed
		PurchaseRareEggsStatusChanged {
			status: bool,
		},
		/// Preorder Eggs status has changed
		PreorderEggsStatusChanged {
			status: bool,
		},
		OverlordChanged {
			old_overlord: Option<T::AccountId>,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		WorldClockAlreadySet,
		SpiritClaimNotAvailable,
		RareEggPurchaseNotAvailable,
		PreorderEggNotAvailable,
		SpiritAlreadyClaimed,
		ClaimVerificationFailed,
		InvalidPurchase,
		NoAvailablePreorderId,
		RaceMintMaxReached,
		CareerMintMaxReached,
		CannotHatchEgg,
		CannotSendFoodToEgg,
		NoFoodAvailable,
		OverlordNotSet,
		RequireOverlordAccount,
		InvalidStatusType,
		SpiritCollectionNotSet,
		SpiritCollectionIdAlreadySet,
		EggCollectionNotSet,
		EggCollectionIdAlreadySet,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T: pallet_uniques::Config<ClassId = CollectionId, InstanceId = NftId>,
	{
		/// Claim a spirit for users that are on the whitelist. This whitelist will consist of a
		/// a serial id and an account id that is signed by the admin account. When a user comes
		/// to claim their spirit, they will provide a serial id & will be validated as an
		/// authenticated claimer
		///
		/// Parameters:
		/// - origin: The origin of the extrinsic.
		/// - serial_id: The serial id of the spirit to be claimed.
		/// - signature: The signature of the account that is claiming the spirit.
		///   //Sr25519Signature
		/// - metadata: The metadata of the account that is claiming the spirit.
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn claim_spirit(
			origin: OriginFor<T>,
			serial_id: SerialId,
			signature: sr25519::Signature,
			metadata: BoundedVec<u8, T::StringLimit>,
		) -> DispatchResult {
			ensure!(CanClaimSpirits::<T>::get(), Error::<T>::SpiritClaimNotAvailable);
			let sender = ensure_signed(origin)?;
			let overlord = Overlord::<T>::get().ok_or(Error::<T>::OverlordNotSet)?;
			// Has Spirit Collection been set
			let spirit_collection_id =
				SpiritCollectionId::<T>::get().ok_or(Error::<T>::SpiritCollectionNotSet)?;
			// Has the SerialId already been claimed
			ensure!(
				!ClaimedSpirits::<T>::contains_key(serial_id),
				Error::<T>::SpiritAlreadyClaimed
			);
			// Check if valid SerialId to claim a spirit
			ensure!(
				Self::verify_claim(sender.clone(), metadata.clone(), signature),
				Error::<T>::ClaimVerificationFailed
			);
			// Mint new Spirit and transfer to sender
			pallet_rmrk_core::Pallet::<T>::mint_nft(
				Origin::<T>::Signed(overlord).into(),
				sender.clone(),
				spirit_collection_id,
				None,
				None,
				metadata,
			)?;
			ClaimedSpirits::<T>::insert(serial_id, true);

			Self::deposit_event(Event::SpiritClaimed { serial_id, owner: sender });

			Ok(())
		}

		/// Buy a rare egg of either type Legendary or Founder. Both Egg types will have a set
		/// price. These will also be limited in quantity and on a first come, first serve basis.
		///
		/// Parameters:
		/// - origin: The origin of the extrinsic.
		/// - egg_type: The type of egg to be purchased.
		/// - race: The race of the egg chosen by the user.
		/// - career: The career of the egg chosen by the user or auto-generated based on metadata
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn buy_rare_egg(
			origin: OriginFor<T>,
			egg_type: EggType,
			race: RaceType,
			career: CareerType,
			metadata: BoundedVec<u8, T::StringLimit>,
		) -> DispatchResult {
			ensure!(CanPurchaseRareEggs::<T>::get(), Error::<T>::RareEggPurchaseNotAvailable);
			let sender = ensure_signed(origin.clone())?;
			let overlord = Overlord::<T>::get().ok_or(Error::<T>::OverlordNotSet)?;
			// Ensure egg collection is set
			let egg_collection_id =
				EggCollectionId::<T>::get().ok_or(Error::<T>::EggCollectionNotSet)?;
			// Get Egg Price based on EggType
			let egg_price = match egg_type {
				EggType::Founder => T::FounderEggPrice::get(),
				EggType::Legendary => T::LegendaryEggPrice::get(),
				_ => return Err(Error::<T>::InvalidPurchase.into()),
			};
			let nft_id = pallet_rmrk_core::NextNftId::<T>::get(egg_collection_id);
			// Check if race and career types have mints left
			Self::has_race_type_left(&race)?;
			Self::has_career_type_left(&career)?;

			// Define EggInfo for storage
			let egg = EggInfo {
				egg_type,
				race: race.clone(),
				career: career.clone(),
				start_hatching: 0,
				hatching_duration: 0,
			};

			// Transfer the amount for the rare Egg NFT then mint the egg
			<T as pallet::Config>::Currency::transfer(
				&sender,
				&overlord,
				egg_price,
				ExistenceRequirement::KeepAlive,
			)?;
			// Mint Egg and transfer Egg to new owner
			pallet_rmrk_core::Pallet::<T>::mint_nft(
				Origin::<T>::Signed(overlord.clone()).into(),
				sender.clone(),
				egg_collection_id,
				None,
				None,
				metadata,
			)?;

			Self::decrement_race_type(race);
			Self::decrement_career_type(career);
			Eggs::<T>::insert(egg_collection_id, nft_id, egg);

			Self::deposit_event(Event::RareEggPurchased {
				collection_id: egg_collection_id,
				nft_id,
				owner: sender,
			});

			Ok(())
		}

		/// Users can pre-order an egg. This will enable users that are whitelisted to be
		/// added to the queue of users that can claim eggs. Those that come after the whitelist
		/// pre-sale will be able to win the chance to acquire an egg based on their choice of
		/// race and career as they will have a limited quantity.
		///
		/// Parameters:
		/// - origin: The origin of the extrinsic preordering the egg
		/// - race: The race that the user has chosen (limited # of races)
		/// - career: The career that the user has chosen (limited # of careers)
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn preorder_egg(
			origin: OriginFor<T>,
			race: RaceType,
			career: CareerType,
			metadata: BoundedVec<u8, T::StringLimit>,
		) -> DispatchResult {
			ensure!(CanPreorderEggs::<T>::get(), Error::<T>::PreorderEggNotAvailable);
			let sender = ensure_signed(origin)?;
			// Check if the race and career have reached their limit
			Self::has_race_type_left(&race)?;
			Self::has_career_type_left(&career)?;
			// Get preorder_id for new preorder
			let preorder_id =
				<PreorderIndex<T>>::try_mutate(|n| -> Result<PreorderId, DispatchError> {
					let id = *n;
					ensure!(id != PreorderId::max_value(), Error::<T>::NoAvailablePreorderId);
					*n = *n + 1;
					Ok(id)
				})?;

			let preorder = PreorderInfo {
				owner: sender.clone(),
				race: race.clone(),
				career: career.clone(),
				metadata,
			};
			// Reserve currency for the preorder at the Normal egg price
			<T as pallet::Config>::Currency::reserve(&sender, T::NormalEggPrice::get())?;

			Self::decrement_race_type(race);
			Self::decrement_career_type(career);
			Preorders::<T>::insert(preorder_id, preorder);

			Self::deposit_event(Event::EggPreordered { owner: sender, preorder_id });

			Ok(())
		}

		/// This is an admin only function that will be called to do a bulk minting of all egg
		/// owners that made selected a race and career that was available based on the quantity
		/// available. Those that did not win an egg will have to claim their refund
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn mint_eggs(origin: OriginFor<T>) -> DispatchResult {
			// Ensure Overlord account makes call
			let sender = ensure_signed(origin)?;
			Self::ensure_overlord(sender.clone())?;
			// Ensure egg collection is set
			let egg_collection_id =
				EggCollectionId::<T>::get().ok_or(Error::<T>::EggCollectionNotSet)?;
			// Iterate through Preorders
			for preorder_id in Preorders::<T>::iter_keys() {
				if let Some(preorder) = Preorders::<T>::take(preorder_id) {
					let egg_price = T::NormalEggPrice::get();
					// Define EggInfo for storage
					let egg = EggInfo {
						egg_type: EggType::Normal,
						race: preorder.race.clone(),
						career: preorder.career.clone(),
						start_hatching: 0,
						hatching_duration: 0,
					};
					// Next NFT ID of Collection
					let nft_id = pallet_rmrk_core::NextNftId::<T>::get(egg_collection_id);

					// Get payment from owner's reserve
					<T as pallet::Config>::Currency::unreserve(&preorder.owner, egg_price);
					<T as pallet::Config>::Currency::transfer(
						&preorder.owner,
						&sender,
						egg_price,
						ExistenceRequirement::KeepAlive,
					)?;
					// Mint Egg and transfer Egg to new owner
					pallet_rmrk_core::Pallet::<T>::mint_nft(
						Origin::<T>::Signed(sender.clone()).into(),
						preorder.owner.clone(),
						egg_collection_id,
						None,
						None,
						preorder.metadata,
					)?;

					Eggs::<T>::insert(egg_collection_id, nft_id, egg);

					Self::deposit_event(Event::EggMinted {
						collection_id: egg_collection_id,
						nft_id,
						owner: preorder.owner,
					});
				}
			}

			Ok(())
		}

		/// Once users have received their eggs and the start hatching event has been triggered,
		/// they can start the hatching process and a timer will start for the egg to hatch at
		/// a designated time. Eggs can reduce their time by being in the top 10 of egg's fed
		/// per era.
		///
		/// Parameters:
		/// - origin: The origin of the extrinsic starting the hatching process
		/// - collection_id: The collection id of the Egg RMRK NFT
		/// - nft_id: The NFT id of the Egg RMRK NFT
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn start_hatching(
			origin: OriginFor<T>,
			collection_id: CollectionId,
			nft_id: NftId,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			Ok(())
		}

		/// Feed another egg to the current egg being hatched. This will reduce the time left to
		/// hatching if the egg is in the top 10 of eggs fed that era.
		///
		/// Parameters:
		/// - origin: The origin of the extrinsic feeding the egg
		/// - collection_id: The collection id of the Egg RMRK NFT
		/// - nft_id: The NFT id of the Egg RMRK NFT
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn feed_egg(
			origin: OriginFor<T>,
			collection_id: CollectionId,
			nft_id: NftId,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			Ok(())
		}

		/// Hatch the egg that is currently being hatched. This will trigger the end of the hatching
		/// process and the egg will be burned. After burning, the user will receive the awakened
		/// Shell RMRK NFT
		///
		/// Parameters:
		/// - origin: The origin of the extrinsic hatching the egg
		/// - collection_id: The collection id of the Egg RMRK NFT
		/// - nft_id: The NFT id of the Egg RMRK NFT
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn hatch_egg(
			origin: OriginFor<T>,
			collection_id: CollectionId,
			nft_id: NftId,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			Ok(())
		}

		/// This is an admin function to update eggs hatch times based on being in the top 10 of
		/// fed eggs within that era
		///
		/// Parameters:
		/// - origin: The origin of the extrinsic updating the eggs hatch times
		/// - collection_id: The collection id of the Egg RMRK NFT
		/// - nft_id: The NFT id of the Egg RMRK NFT
		/// - reduced_time: The amount of time the egg will be reduced by
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn update_hatch_time(
			origin: OriginFor<T>,
			collection_id: CollectionId,
			nft_id: NftId,
			reduced_time: u64,
		) -> DispatchResult {
			// Ensure OverlordOrigin makes call
			let sender = ensure_signed(origin)?;
			Self::ensure_overlord(sender)?;

			Ok(())
		}

		/// Privileged function set the Overlord Admin account of Phala World
		///
		/// Parameters:
		/// - origin: Expected to be called by `OverlordOrigin`
		/// - new_overlord: T::AccountId
		#[pallet::weight(0)]
		pub fn set_overlord(
			origin: OriginFor<T>,
			new_overlord: T::AccountId,
		) -> DispatchResultWithPostInfo {
			// This is a public call, so we ensure that the origin is some signed account.
			ensure_root(origin)?;
			let old_overlord = <Overlord<T>>::get();

			Overlord::<T>::put(&new_overlord);
			Self::deposit_event(Event::OverlordChanged { old_overlord });
			// GameOverlord user does not pay a fee
			Ok(Pays::No.into())
		}

		/// Phala World Zero Day is set to begin the tracking of the official time starting at the
		/// current timestamp when `initialize_world_clock` is called by the `Overlord`
		///
		/// Parameters:
		/// `origin`: Expected to be called by `Overlord` admin account
		#[pallet::weight(0)]
		pub fn initialize_world_clock(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			// Ensure Overlord account makes call
			let sender = ensure_signed(origin)?;
			Self::ensure_overlord(sender)?;
			// Ensure ZeroDay is None as this can only be set once
			ensure!(Self::zero_day() == None, Error::<T>::WorldClockAlreadySet);

			let zero_day = T::Time::now().as_secs();

			ZeroDay::<T>::put(zero_day);
			Self::deposit_event(Event::WorldClockStarted { start_time: zero_day });

			Ok(Pays::No.into())
		}

		/// Privileged function to set the status for one of the defined StatusTypes like
		/// ClaimSpirits, PurchaseRareEggs, or PreorderEggs to enable functionality in Phala World
		///
		/// Parameters:
		/// - `origin` - Expected Overlord admin account to set the status
		/// - `status` - `bool` to set the status to
		/// - `status_type` - `StatusType` to set the status for
		#[pallet::weight(0)]
		pub fn set_status_type(
			origin: OriginFor<T>,
			status: bool,
			status_type: StatusType,
		) -> DispatchResultWithPostInfo {
			// Ensure Overlord account makes call
			let sender = ensure_signed(origin)?;
			Self::ensure_overlord(sender)?;
			// Match StatusType and call helper function to set status
			match status_type {
				StatusType::ClaimSpirits => Self::set_claim_spirits_status(status)?,
				StatusType::PurchaseRareEggs => Self::set_purchase_rare_eggs_status(status)?,
				StatusType::PreorderEggs => Self::set_preorder_eggs_status(status)?,
			}
			Ok(Pays::No.into())
		}

		/// Privileged function to set the collection id for the Spirits collection
		///
		/// Parameters:
		/// - `origin` - Expected Overlord admin account to set the Spirit Collection ID
		/// - `collection_id` - Collection ID of the Spirit Collection
		#[pallet::weight(0)]
		pub fn set_spirit_collection_id(
			origin: OriginFor<T>,
			collection_id: CollectionId,
		) -> DispatchResultWithPostInfo {
			// Ensure Overlord account makes call
			let sender = ensure_signed(origin)?;
			Self::ensure_overlord(sender)?;
			// If Spirit Collection ID is greater than 0 then the collection ID was already set
			ensure!(
				SpiritCollectionId::<T>::get().is_none(),
				Error::<T>::SpiritCollectionIdAlreadySet
			);
			<SpiritCollectionId<T>>::put(collection_id);

			Self::deposit_event(Event::SpiritCollectionIdSet { collection_id });

			Ok(Pays::No.into())
		}

		/// Privileged function to set the collection id for the Egg collection
		///
		/// Parameters:
		/// - `origin` - Expected Overlord admin account to set the Egg Collection ID
		/// - `collection_id` - Collection ID of the Egg Collection
		#[pallet::weight(0)]
		pub fn set_egg_collection_id(
			origin: OriginFor<T>,
			collection_id: CollectionId,
		) -> DispatchResultWithPostInfo {
			// Ensure Overlord account makes call
			let sender = ensure_signed(origin)?;
			Self::ensure_overlord(sender)?;
			// If Egg Collection ID is greater than 0 then the collection ID was already set
			ensure!(EggCollectionId::<T>::get().is_none(), Error::<T>::EggCollectionIdAlreadySet);
			<EggCollectionId<T>>::put(collection_id);

			Self::deposit_event(Event::EggCollectionIdSet { collection_id });

			Ok(Pays::No.into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Verify the claim status of an Account that has claimed a spirit. Serialize the evidence with
	/// the provided account and metadata and verify the against the expected results by validating
	/// against the Overlord account used to sign and generate the whitelisted user's SerialId
	///
	/// Parameters:
	/// - claimer: AccountId of the account claiming the spirit
	/// - metadata: Metadata passed in associated with the claimer
	/// - signature: Signature of the claimer
	pub fn verify_claim(
		claimer: T::AccountId,
		metadata: BoundedVec<u8, T::StringLimit>,
		signature: sr25519::Signature,
	) -> bool {
		// Serialize the evidence
		let msg = Encode::encode(&(claimer, metadata));
		if let Some(overlord) = <Overlord<T>>::get() {
			let encode_overlord = T::AccountId::encode(&overlord);
			let h256_overlord = H256::from_slice(&encode_overlord);
			let overlord_key = sr25519::Public::from_h256(h256_overlord);
			// verify claim
			sp_io::crypto::sr25519_verify(&signature, &msg, &overlord_key)
		} else {
			false
		}
	}

	/// Helper function to ensure Overlord account is the sender
	///
	/// Parameters:
	/// - `sender`: Account origin that made the call to check if Overlord account
	fn ensure_overlord(sender: T::AccountId) -> DispatchResult {
		ensure!(
			Self::overlord().map_or(false, |k| sender == k),
			Error::<T>::RequireOverlordAccount
		);
		Ok(())
	}

	/// Set Spirit Claims with the Overlord admin Account to allow users to claim their
	/// Spirits through the `claim_spirits()` function
	///
	/// Parameters:
	/// - `status`: Status to set CanClaimSpirits StorageValue
	fn set_claim_spirits_status(status: bool) -> DispatchResult {
		<CanClaimSpirits<T>>::put(status);

		Self::deposit_event(Event::ClaimSpiritStatusChanged { status });

		Ok(())
	}

	/// Set Rare Eggs status for purchase with the Overlord Admin Account to allow
	/// users to purchase either Founder or Legendary Eggs
	///
	/// Parameters:
	/// `status`: Status to set CanPurchaseRareEggs StorageValue
	fn set_purchase_rare_eggs_status(status: bool) -> DispatchResult {
		<CanPurchaseRareEggs<T>>::put(status);

		Self::deposit_event(Event::PurchaseRareEggsStatusChanged { status });

		Ok(())
	}

	/// Set status of Preordering eggs with the Overlord Admin Account to allow
	/// users to preorder eggs through the `preorder_egg()` function
	///
	/// Parameters:
	/// - `status`: Status to set CanPreorderEggs StorageValue
	fn set_preorder_eggs_status(status: bool) -> DispatchResult {
		<CanPreorderEggs<T>>::put(status);

		Self::deposit_event(Event::PreorderEggsStatusChanged { status });

		Ok(())
	}

	/// Decrement RaceType count for the `race`
	///
	/// Parameters:
	/// - `race`: The Race to increment count
	fn decrement_race_type(race: RaceType) -> DispatchResult {
		RaceTypeLeft::<T>::mutate(race, |race_count| {
			*race_count = *race_count - 1;
			*race_count
		});

		Ok(())
	}

	/// Decrement CareerType count for the `career`
	///
	/// Parameters:
	/// - `career`: The Career to increment count
	fn decrement_career_type(career: CareerType) -> DispatchResult {
		CareerTypeLeft::<T>::mutate(career, |career_count| {
			*career_count = *career_count - 1;
			*career_count
		});

		Ok(())
	}

	/// Increment RaceType count for the `race`
	///
	/// Parameters:
	/// - `race`: The Race to increment count
	fn increment_race_type(race: RaceType) -> DispatchResult {
		RaceTypeLeft::<T>::mutate(race, |race_count| {
			*race_count = *race_count + 1;
			*race_count
		});

		Ok(())
	}

	/// Increment CareerType count for the `career`
	///
	/// Parameters:
	/// - `career`: The Career to increment count
	fn increment_career_type(career: CareerType) -> DispatchResult {
		CareerTypeLeft::<T>::mutate(career, |career_count| {
			*career_count = *career_count + 1;
			*career_count
		});

		Ok(())
	}

	/// Verify if the chosen Race has reached the max limit
	///
	/// Parameters:
	/// - `race`: The Race to check
	fn has_race_type_left(race: &RaceType) -> DispatchResult {
		ensure!(RaceTypeLeft::<T>::get(race) > 0, Error::<T>::RaceMintMaxReached);
		Ok(())
	}

	/// Verify if the chosen a Career has reached the max limit
	///
	/// Parameters:
	/// - `career`: The Career to check
	fn has_career_type_left(career: &CareerType) -> DispatchResult {
		ensure!(CareerTypeLeft::<T>::get(career) > 0, Error::<T>::CareerMintMaxReached);
		Ok(())
	}
}
