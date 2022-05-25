//! Phala World NFT Sale Pallet

use frame_support::{
	ensure,
	traits::{
		tokens::{nonfungibles::InspectEnumerable, ExistenceRequirement},
		Currency, UnixTime,
	},
	transactional, BoundedVec,
};
use frame_system::{ensure_signed, pallet_prelude::*, Origin};

use codec::Encode;
use sp_core::{sr25519, H256};
use sp_runtime::DispatchResult;
use sp_std::prelude::*;

pub use pallet_rmrk_core::types::*;
pub use pallet_rmrk_market;

use rmrk_traits::{
	career::CareerType, message::Purpose, origin_of_shell::OriginOfShellType, primitives::*,
	race::RaceType, status_type::StatusType, NftSaleInfo, OverlordMessage, PreorderInfo,
};

pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::ReservableCurrency};

	type PreorderInfoOf<T> = PreorderInfo<
		<T as frame_system::Config>::AccountId,
		BoundedVec<u8, <T as pallet_uniques::Config>::StringLimit>,
	>;

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
		/// Minimum amount of PHA to claim a Spirit
		#[pallet::constant]
		type MinBalanceToClaimSpirit: Get<BalanceOf<Self>>;
		/// Price of Legendary Origin of Shell Price
		#[pallet::constant]
		type LegendaryOriginOfShellPrice: Get<BalanceOf<Self>>;
		/// Price of Magic Origin of Shell Price
		#[pallet::constant]
		type MagicOriginOfShellPrice: Get<BalanceOf<Self>>;
		/// Price of Prime Origin of Shell Price
		#[pallet::constant]
		type PrimeOriginOfShellPrice: Get<BalanceOf<Self>>;
		/// Max mint per Race
		#[pallet::constant]
		type IterLimit: Get<u32>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Preorder index that is the key to the Preorders StorageMap
	#[pallet::storage]
	#[pallet::getter(fn preorder_index)]
	pub type PreorderIndex<T: Config> = StorageValue<_, PreorderId, ValueQuery>;

	/// Preorder info map for user preorders
	#[pallet::storage]
	#[pallet::getter(fn preorders)]
	pub type Preorders<T: Config> = StorageMap<_, Twox64Concat, PreorderId, PreorderInfoOf<T>>;

	/// Origin of Shells inventory
	#[pallet::storage]
	#[pallet::getter(fn origin_of_shells_inventory)]
	pub type OriginOfShellsInventory<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		OriginOfShellType,
		Blake2_128Concat,
		RaceType,
		NftSaleInfo,
	>;

	/// Phala World Zero Day `BlockNumber` this will be used to determine Eras
	#[pallet::storage]
	#[pallet::getter(fn zero_day)]
	pub(super) type ZeroDay<T: Config> = StorageValue<_, u64>;

	/// The current Era from the initial ZeroDay BlockNumber
	#[pallet::storage]
	#[pallet::getter(fn era)]
	pub type Era<T: Config> = StorageValue<_, EraId, ValueQuery>;

	/// Spirits can be claimed
	#[pallet::storage]
	#[pallet::getter(fn can_claim_spirits)]
	pub type CanClaimSpirits<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Rare Origin of Shells can be purchased
	#[pallet::storage]
	#[pallet::getter(fn can_purchase_rare_origin_of_shells)]
	pub type CanPurchaseRareOriginOfShells<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Origin of Shells can be purchased by whitelist
	#[pallet::storage]
	#[pallet::getter(fn can_purchase_prime_origin_of_shells)]
	pub type CanPurchasePrimeOriginOfShells<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Origin of Shells can be preordered
	#[pallet::storage]
	#[pallet::getter(fn can_preorder_origin_of_shells)]
	pub type CanPreorderOriginOfShells<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Last Day of Sale any Origin of Shell can be purchased
	#[pallet::storage]
	#[pallet::getter(fn last_day_of_sale)]
	pub type LastDayOfSale<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Spirit Collection ID
	#[pallet::storage]
	#[pallet::getter(fn spirit_collection_id)]
	pub type SpiritCollectionId<T: Config> = StorageValue<_, CollectionId, OptionQuery>;

	/// Origin of Shell Collection ID
	#[pallet::storage]
	#[pallet::getter(fn origin_of_shell_collection_id)]
	pub type OriginOfShellCollectionId<T: Config> = StorageValue<_, CollectionId, OptionQuery>;

	/// Career StorageMap count
	#[pallet::storage]
	#[pallet::getter(fn career_type_count)]
	pub type CareerTypeCount<T: Config> = StorageMap<_, Twox64Concat, CareerType, u32, ValueQuery>;

	/// Origin of Shells Inventory has been initialized
	#[pallet::storage]
	#[pallet::getter(fn is_origin_of_shells_inventory_set)]
	pub type IsOriginOfShellsInventorySet<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Overlord Admin account of Phala World
	#[pallet::storage]
	pub(super) type Overlord<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_n: T::BlockNumber) {
			if let Some(zero_day) = <ZeroDay<T>>::get() {
				let current_time = T::Time::now().as_secs();
				if current_time > zero_day {
					let secs_since_zero_day = current_time - zero_day;
					let current_era = <Era<T>>::get();
					if secs_since_zero_day / T::SecondsPerEra::get() > current_era {
						let new_era = Era::<T>::mutate(|era| {
							*era += 1;
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
		/// bool for if a Rare Origin of Shell can be purchased
		pub can_purchase_rare_origin_of_shells: bool,
		/// bool for Prime Origin of Shell purchases through whitelist
		pub can_purchase_prime_origin_of_shells: bool,
		/// bool for if an Origin of Shell can be preordered
		pub can_preorder_origin_of_shells: bool,
		/// bool for the last day of sale for Origin of Shell
		pub last_day_of_sale: bool,
		/// CollectionId of Spirit Collection
		pub spirit_collection_id: Option<CollectionId>,
		/// CollectionId of Origin of Shell Collection
		pub origin_of_shell_collection_id: Option<CollectionId>,
		/// Is Origin of Shells Inventory set?
		pub is_origin_of_shells_inventory_set: bool,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				zero_day: None,
				overlord: None,
				era: 0,
				can_claim_spirits: false,
				can_purchase_rare_origin_of_shells: false,
				can_purchase_prime_origin_of_shells: false,
				can_preorder_origin_of_shells: false,
				last_day_of_sale: false,
				spirit_collection_id: None,
				origin_of_shell_collection_id: None,
				is_origin_of_shells_inventory_set: false,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T>
	where
		T: pallet_uniques::Config<ClassId = CollectionId, InstanceId = NftId>,
	{
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
			let can_purchase_rare_origin_of_shells = self.can_purchase_rare_origin_of_shells;
			<CanPurchaseRareOriginOfShells<T>>::put(can_purchase_rare_origin_of_shells);
			let can_purchase_prime_origin_of_shells = self.can_purchase_prime_origin_of_shells;
			<CanPurchasePrimeOriginOfShells<T>>::put(can_purchase_prime_origin_of_shells);
			let can_preorder_origin_of_shells = self.can_preorder_origin_of_shells;
			<CanPreorderOriginOfShells<T>>::put(can_preorder_origin_of_shells);
			let last_day_of_sale = self.last_day_of_sale;
			<LastDayOfSale<T>>::put(last_day_of_sale);
			if let Some(spirit_collection_id) = self.spirit_collection_id {
				<SpiritCollectionId<T>>::put(spirit_collection_id);
			}
			if let Some(origin_of_shell_collection_id) = self.origin_of_shell_collection_id {
				<OriginOfShellCollectionId<T>>::put(origin_of_shell_collection_id);
			}
			let is_origin_of_shells_inventory_set = self.is_origin_of_shells_inventory_set;
			<IsOriginOfShellsInventorySet<T>>::put(is_origin_of_shells_inventory_set);
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
		/// Spirit has been claimed
		SpiritClaimed {
			owner: T::AccountId,
			collection_id: CollectionId,
			nft_id: NftId,
		},
		/// A chance to get an Origin of Shell through preorder
		OriginOfShellPreordered {
			owner: T::AccountId,
			preorder_id: PreorderId,
		},
		/// Origin of Shell minted from the preorder
		OriginOfShellMinted {
			origin_of_shell_type: OriginOfShellType,
			collection_id: CollectionId,
			nft_id: NftId,
			owner: T::AccountId,
			race: RaceType,
			career: CareerType,
		},
		/// Spirit collection id was set
		SpiritCollectionIdSet {
			collection_id: CollectionId,
		},
		/// Origin of Shell collection id was set
		OriginOfShellCollectionIdSet {
			collection_id: CollectionId,
		},
		/// Origin of Shell inventory updated
		OriginOfShellInventoryUpdated {
			origin_of_shell_type: OriginOfShellType,
		},
		/// Spirit Claims status has changed
		ClaimSpiritStatusChanged {
			status: bool,
		},
		/// Purchase Rare Origin of Shells status has changed
		PurchaseRareOriginOfShellsStatusChanged {
			status: bool,
		},
		/// Purchase Prime Origin of Shells status changed
		PurchasePrimeOriginOfShellsStatusChanged {
			status: bool,
		},
		/// Preorder Origin of Shells status has changed
		PreorderOriginOfShellsStatusChanged {
			status: bool,
		},
		/// Chosen preorder was minted to owner
		ChosenPreorderMinted {
			preorder_id: PreorderId,
			owner: T::AccountId,
		},
		/// Not chosen preorder was refunded to owner
		NotChosenPreorderRefunded {
			preorder_id: PreorderId,
			owner: T::AccountId,
		},
		/// Last Day of Sale status has changed
		LastDayOfSaleStatusChanged {
			status: bool,
		},
		OverlordChanged {
			old_overlord: Option<T::AccountId>,
			new_overlord: T::AccountId,
		},
		/// Origin of Shells Inventory was set
		OriginOfShellsInventoryWasSet {
			status: bool,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		WorldClockAlreadySet,
		SpiritClaimNotAvailable,
		RareOriginOfShellPurchaseNotAvailable,
		PrimeOriginOfShellPurchaseNotAvailable,
		PreorderOriginOfShellNotAvailable,
		PreorderClaimNotAvailable,
		SpiritAlreadyClaimed,
		InvalidSpiritClaim,
		InvalidMetadata,
		MustOwnSpiritToPurchase,
		OriginOfShellAlreadyPurchased,
		BelowMinimumBalanceThreshold,
		WhitelistVerificationFailed,
		InvalidPurchase,
		NoAvailablePreorderId,
		PreorderClaimNotDetected,
		RefundClaimNotDetected,
		PreorderIsPending,
		PreordersCorrupted,
		NotPreorderOwner,
		RaceMintMaxReached,
		OverlordNotSet,
		RequireOverlordAccount,
		InvalidStatusType,
		WrongOriginOfShellType,
		SpiritCollectionNotSet,
		SpiritCollectionIdAlreadySet,
		OriginOfShellCollectionNotSet,
		OriginOfShellCollectionIdAlreadySet,
		OriginOfShellInventoryCorrupted,
		OriginOfShellInventoryAlreadySet,
		UnableToAddAttributes,
		KeyTooLong,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T: pallet_uniques::Config<ClassId = CollectionId, InstanceId = NftId>,
	{
		/// Claim a spirit for any account with at least 10 PHA in their account
		///
		/// Parameters:
		/// - origin: The origin of the extrinsic.
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn claim_spirit(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;
			ensure!(CanClaimSpirits::<T>::get(), Error::<T>::SpiritClaimNotAvailable);
			let overlord = Self::overlord()?;
			// Check Balance has minimum required
			ensure!(
				<T as pallet::Config>::Currency::can_reserve(
					&sender,
					T::MinBalanceToClaimSpirit::get()
				),
				Error::<T>::BelowMinimumBalanceThreshold
			);
			// Mint Spirit NFT
			Self::do_mint_spirit_nft(overlord, sender)?;

			Ok(())
		}

		/// Redeem spirit function is called when an account has a `SpiritClaimTicket` that enables
		/// an account to acquire a Spirit NFT without the 10 PHA minimum requirement, such that,
		/// the account has a valid `SpiritClaimTicket` signed by the Overlord admin account.
		///
		/// Parameters:
		/// - origin: The origin of the extrinsic.
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn redeem_spirit(
			origin: OriginFor<T>,
			signature: sr25519::Signature,
		) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;
			ensure!(CanClaimSpirits::<T>::get(), Error::<T>::SpiritClaimNotAvailable);
			let overlord = Self::overlord()?;
			// verify the claim ticket
			ensure!(
				Self::verify_claim(&overlord, &sender, signature, Purpose::RedeemSpirit),
				Error::<T>::InvalidSpiritClaim
			);
			// Mint Spirit NFT
			Self::do_mint_spirit_nft(overlord, sender)?;

			Ok(())
		}

		/// Buy a rare origin_of_shell of either type Magic or Legendary. Both Origin of Shell types
		/// will have a set price. These will also be limited in quantity and on a first come, first
		/// serve basis.
		///
		/// Parameters:
		/// - origin: The origin of the extrinsic.
		/// - origin_of_shell_type: The type of origin_of_shell to be purchased.
		/// - race: The race of the origin_of_shell chosen by the user.
		/// - career: The career of the origin_of_shell chosen by the user or auto-generated based
		///   on metadata
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn buy_rare_origin_of_shell(
			origin: OriginFor<T>,
			origin_of_shell_type: OriginOfShellType,
			race: RaceType,
			career: CareerType,
		) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;
			ensure!(
				CanPurchaseRareOriginOfShells::<T>::get(),
				Error::<T>::RareOriginOfShellPurchaseNotAvailable
			);
			let overlord = Self::overlord()?;
			// Get Origin of Shell Price based on Origin of ShellType
			let origin_of_shell_price = match origin_of_shell_type {
				OriginOfShellType::Legendary => T::LegendaryOriginOfShellPrice::get(),
				OriginOfShellType::Magic => T::MagicOriginOfShellPrice::get(),
				_ => return Err(Error::<T>::InvalidPurchase.into()),
			};
			// Mint origin of shell
			Self::do_mint_origin_of_shell_nft(
				overlord,
				sender,
				origin_of_shell_type,
				race,
				career,
				origin_of_shell_price,
				!LastDayOfSale::<T>::get(),
			)?;

			Ok(())
		}

		/// Accounts that have been whitelisted can purchase an Origin of Shell. The only Origin of
		/// Shell type available for this purchase are Prime
		///
		/// Parameters:
		/// - origin: The origin of the extrinsic purchasing the Prime Origin of Shell
		/// - message: OverlordMessage with account and purpose
		/// - signature: The signature of the account that is claiming the spirit.
		/// - race: The race that the user has chosen (limited # of races)
		/// - career: The career that the user has chosen (unlimited careers)
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn buy_prime_origin_of_shell(
			origin: OriginFor<T>,
			signature: sr25519::Signature,
			race: RaceType,
			career: CareerType,
		) -> DispatchResult {
			let sender = ensure_signed(origin.clone())?;
			let is_last_day_of_sale = LastDayOfSale::<T>::get();
			ensure!(
				CanPurchasePrimeOriginOfShells::<T>::get() || is_last_day_of_sale,
				Error::<T>::PrimeOriginOfShellPurchaseNotAvailable
			);

			let overlord = Self::overlord()?;
			// Check if valid message purpose is 'BuyPrimeOriginOfShells' and verify whitelist
			// account
			ensure!(
				Self::verify_claim(&overlord, &sender, signature, Purpose::BuyPrimeOriginOfShells) ||
					is_last_day_of_sale,
				Error::<T>::WhitelistVerificationFailed
			);
			// Get Prime Origin of Shell price
			let origin_of_shell_price = T::PrimeOriginOfShellPrice::get();
			// Mint origin of shell
			Self::do_mint_origin_of_shell_nft(
				overlord,
				sender,
				OriginOfShellType::Prime,
				race,
				career,
				origin_of_shell_price,
				!is_last_day_of_sale,
			)?;

			Ok(())
		}

		/// Users can pre-order an Origin of Shell. This will enable users that are non-whitelisted
		/// to be added to the queue of users that can claim Origin of Shells. Those that come after
		/// the whitelist pre-sale will be able to win the chance to acquire an Origin of Shell
		/// based on their choice of race and career as they will have a limited quantity.
		///
		/// Parameters:
		/// - origin: The origin of the extrinsic preordering the origin_of_shell
		/// - race: The race that the user has chosen (limited # of races)
		/// - career: The career that the user has chosen (limited # of careers)
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn preorder_origin_of_shell(
			origin: OriginFor<T>,
			race: RaceType,
			career: CareerType,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(
				CanPreorderOriginOfShells::<T>::get(),
				Error::<T>::PreorderOriginOfShellNotAvailable
			);
			// Has Spirit Collection been set
			let spirit_collection_id = Self::get_spirit_collection_id()?;
			// Must have origin of shell collection created
			ensure!(
				Self::owns_nft_in_collection(&sender, spirit_collection_id),
				Error::<T>::OriginOfShellAlreadyPurchased
			);

			// Get preorder_id for new preorder
			let preorder_id =
				<PreorderIndex<T>>::try_mutate(|n| -> Result<PreorderId, DispatchError> {
					let id = *n;
					ensure!(id != PreorderId::max_value(), Error::<T>::NoAvailablePreorderId);
					*n += 1;
					Ok(id)
				})?;
			// Verify metadata
			let metadata = Self::get_empty_metadata();
			let preorder = PreorderInfo { owner: sender.clone(), race, career, metadata };
			// Reserve currency for the preorder at the Prime origin_of_shell price
			<T as pallet::Config>::Currency::reserve(&sender, T::PrimeOriginOfShellPrice::get())?;

			Preorders::<T>::insert(preorder_id, preorder);

			Self::deposit_event(Event::OriginOfShellPreordered { owner: sender, preorder_id });

			Ok(())
		}

		/// This is an admin only function that will mint the list of `Chosen` preorders and will
		/// mint the Origin of Shell NFT to the preorder owner.
		///
		/// Parameters:
		/// `origin`: Expected to come from Overlord admin account
		/// `preorders`: Vec of Preorder IDs that were `Chosen`
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn mint_chosen_preorders(
			origin: OriginFor<T>,
			preorders: Vec<PreorderId>,
		) -> DispatchResult {
			// Ensure Overlord account makes call
			let sender = ensure_signed(origin)?;
			Self::ensure_overlord(sender.clone())?;
			// Get price of prime origin of shell
			let origin_of_shell_price = T::PrimeOriginOfShellPrice::get();
			// Get iter limit
			let iter_limit = T::IterLimit::get();
			let mut index = 0;
			// Iterate through the preorder_statuses Vec
			for preorder_id in preorders {
				if index < iter_limit {
					// Get the chosen preorder
					let preorder_info = Preorders::<T>::take(preorder_id)
						.ok_or(Error::<T>::NoAvailablePreorderId)?;
					// Get owner of preorder
					let preorder_owner = preorder_info.owner.clone();
					// Unreserve from owner account
					<T as pallet::Config>::Currency::unreserve(
						&preorder_owner,
						origin_of_shell_price,
					);
					// Mint origin of shell
					Self::do_mint_origin_of_shell_nft(
						sender.clone(),
						preorder_owner.clone(),
						OriginOfShellType::Prime,
						preorder_info.race,
						preorder_info.career,
						origin_of_shell_price,
						false,
					)?;

					Self::deposit_event(Event::ChosenPreorderMinted {
						preorder_id,
						owner: preorder_owner,
					});
					index += 1;
				} else {
					// Break from iterator to ensure block size doesn't grow too large. Re-call this
					// function and it will start from where it left off.
					break
				}
			}

			Ok(())
		}

		/// This is an admin only function that will be used to refund the preorders that were not
		/// selected during the preorders drawing.
		///
		/// Parameters:
		/// `origin`: Expected to come from Overlord admin account
		/// `preorders`: Preorder ids of the not chosen preorders
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		#[transactional]
		pub fn refund_not_chosen_preorders(
			origin: OriginFor<T>,
			preorders: Vec<PreorderId>,
		) -> DispatchResult {
			// Ensure Overlord account makes call
			let sender = ensure_signed(origin)?;
			Self::ensure_overlord(sender)?;
			// Get price of prime origin of shell
			let origin_of_shell_price = T::PrimeOriginOfShellPrice::get();
			// Get iter limit
			let iter_limit = T::IterLimit::get();
			let mut index = 0;
			// Iterate through the preorder_statuses Vec
			for preorder_id in preorders {
				if index < iter_limit {
					// Get the PreorderInfoOf<T>
					let preorder_info = Preorders::<T>::take(preorder_id)
						.ok_or(Error::<T>::NoAvailablePreorderId)?;
					// Refund reserved currency back to owner account
					<T as pallet::Config>::Currency::unreserve(
						&preorder_info.owner,
						origin_of_shell_price,
					);

					Self::deposit_event(Event::NotChosenPreorderRefunded {
						preorder_id,
						owner: preorder_info.owner,
					});
					index += 1;
				} else {
					// Break from iterator to ensure block size doesn't grow too large. Re-call this
					// function and it will start from where it left off.
					break
				}
			}

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
			T::OverlordOrigin::ensure_origin(origin)?;
			let old_overlord = <Overlord<T>>::get();

			Overlord::<T>::put(&new_overlord);
			Self::deposit_event(Event::OverlordChanged { old_overlord, new_overlord });
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
		/// ClaimSpirits, PurchaseRareOriginOfShells, or PreorderOriginOfShells to enable
		/// functionality in Phala World
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
				StatusType::PurchaseRareOriginOfShells =>
					Self::set_purchase_rare_origin_of_shells_status(status)?,
				StatusType::PurchasePrimeOriginOfShells =>
					Self::set_purchase_prime_origin_of_shells_status(status)?,
				StatusType::PreorderOriginOfShells =>
					Self::set_preorder_origin_of_shells_status(status)?,
				StatusType::LastDayOfSale => Self::set_last_day_of_sale_status(status)?,
			}
			Ok(Pays::No.into())
		}

		/// Initialize the settings for the non-whitelist preorder period amount of races &
		/// giveaways available for the Origin of Shell NFTs. This is a privileged function and can
		/// only be executed by the Overlord account. This will call the helper function
		/// `set_initial_origin_of_shell_inventory`
		///
		/// Parameters:
		/// - `origin` - Expected Overlord admin account
		#[pallet::weight(0)]
		pub fn init_origin_of_shell_type_counts(origin: OriginFor<T>) -> DispatchResult {
			// Ensure Overlord account makes call
			let sender = ensure_signed(origin)?;
			Self::ensure_overlord(sender)?;
			// Call helper function
			Self::set_initial_origin_of_shell_inventory()?;
			Self::deposit_event(Event::OriginOfShellsInventoryWasSet { status: true });
			Ok(())
		}

		/// Update for the non-whitelist preorder period amount of races & giveaways available for
		/// the Origin of Shell NFTs. This is a privileged function and can only be executed by the
		/// Overlord account. Update the OriginOfShellInventory counts by incrementing them based on
		/// the defined counts
		///
		/// Parameters:
		/// - `origin` - Expected Overlord admin account
		/// - `origin_of_shell_type` - Type of Origin of Shell
		/// - `for_sale_count` - Number of Origin of Shells for sale
		/// - `giveaway_count` - Number of Origin of Shells for giveaways
		/// - `reserve_count` - Number of Origin of Shells to be reserved
		#[pallet::weight(0)]
		pub fn update_origin_of_shell_type_counts(
			origin: OriginFor<T>,
			origin_of_shell_type: OriginOfShellType,
			for_sale_count: u32,
			giveaway_count: u32,
		) -> DispatchResult {
			// Ensure Overlord account makes call
			let sender = ensure_signed(origin)?;
			Self::ensure_overlord(sender)?;
			// Ensure they are updating the OriginOfShellType::Prime
			ensure!(
				origin_of_shell_type == OriginOfShellType::Prime,
				Error::<T>::WrongOriginOfShellType
			);
			// Mutate the existing storage for the Prime Origin of Shells
			Self::update_nft_sale_info(
				origin_of_shell_type,
				RaceType::AISpectre,
				for_sale_count,
				giveaway_count,
			);
			Self::update_nft_sale_info(
				origin_of_shell_type,
				RaceType::Cyborg,
				for_sale_count,
				giveaway_count,
			);
			Self::update_nft_sale_info(
				origin_of_shell_type,
				RaceType::Pandroid,
				for_sale_count,
				giveaway_count,
			);
			Self::update_nft_sale_info(
				origin_of_shell_type,
				RaceType::XGene,
				for_sale_count,
				giveaway_count,
			);

			Self::deposit_event(Event::OriginOfShellInventoryUpdated { origin_of_shell_type });

			Ok(())
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

		/// Privileged function to set the collection id for the Origin of Shell collection
		///
		/// Parameters:
		/// - `origin` - Expected Overlord admin account to set the Origin of Shell Collection ID
		/// - `collection_id` - Collection ID of the Origin of Shell Collection
		#[pallet::weight(0)]
		pub fn set_origin_of_shell_collection_id(
			origin: OriginFor<T>,
			collection_id: CollectionId,
		) -> DispatchResultWithPostInfo {
			// Ensure Overlord account makes call
			let sender = ensure_signed(origin)?;
			Self::ensure_overlord(sender)?;
			// If Origin of Shell Collection ID is greater than 0 then the collection ID was already
			// set
			ensure!(
				OriginOfShellCollectionId::<T>::get().is_none(),
				Error::<T>::OriginOfShellCollectionIdAlreadySet
			);
			<OriginOfShellCollectionId<T>>::put(collection_id);

			Self::deposit_event(Event::OriginOfShellCollectionIdSet { collection_id });

			Ok(Pays::No.into())
		}
	}
}

impl<T: Config> Pallet<T>
where
	T: pallet_uniques::Config<ClassId = CollectionId, InstanceId = NftId>,
{
	/// Verify the sender making the claim is the Account signed by the Overlord admin account and
	/// verify the purpose of the `OverlordMessage` which will be either `RedeemSpirit` or
	/// `BuyPrimeOriginOfShells`
	///
	/// Parameters:
	/// - `overlord`: Overlord admin account
	/// - `sender`: Sender that redeemed the claim
	/// - `signature`: sr25519::Signature of the expected account making the claim
	/// - `message`: OverlordMessage that contains the account and purpose
	/// - `purpose`: Expected Purpose
	pub fn verify_claim(
		overlord: &T::AccountId,
		sender: &T::AccountId,
		signature: sr25519::Signature,
		purpose: Purpose,
	) -> bool {
		let message = OverlordMessage { account: sender.clone(), purpose };
		let encoded_message = Encode::encode(&message);
		let encode_overlord = T::AccountId::encode(overlord);
		let h256_overlord = H256::from_slice(&encode_overlord);
		let overlord_key = sr25519::Public::from_h256(h256_overlord);
		// verify claim
		sp_io::crypto::sr25519_verify(&signature, &encoded_message, &overlord_key)
	}

	/// Helper function to ensure Overlord account is the sender
	///
	/// Parameters:
	/// - `sender`: Account origin that made the call to check if Overlord account
	pub(crate) fn ensure_overlord(sender: T::AccountId) -> DispatchResult {
		ensure!(
			Self::overlord().map_or(false, |k| sender == k),
			Error::<T>::RequireOverlordAccount
		);
		Ok(())
	}

	/// Helper function to get the Overlord admin account
	pub(crate) fn overlord() -> Result<T::AccountId, Error<T>> {
		Overlord::<T>::get().ok_or(Error::<T>::OverlordNotSet)
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

	/// Set Rare Origin of Shells status for purchase with the Overlord Admin Account to allow
	/// users to purchase either Legendary or Magic Origin of Shells
	///
	/// Parameters:
	/// `status`: Status to set CanPurchaseRareOriginOfShells StorageValue
	fn set_purchase_rare_origin_of_shells_status(status: bool) -> DispatchResult {
		<CanPurchaseRareOriginOfShells<T>>::put(status);

		Self::deposit_event(Event::PurchaseRareOriginOfShellsStatusChanged { status });

		Ok(())
	}

	/// Set Prime Origin of Shells status for purchase with the Overlord Admin Account to allow
	/// users to purchase Prime Origin of Shells
	///
	/// Parameters:
	/// `status`: Status to set CanPurchaseOriginOfShellsWhitelist StorageValue
	fn set_purchase_prime_origin_of_shells_status(status: bool) -> DispatchResult {
		<CanPurchasePrimeOriginOfShells<T>>::put(status);

		Self::deposit_event(Event::PurchasePrimeOriginOfShellsStatusChanged { status });

		Ok(())
	}

	/// Set status of Preordering origin_of_shells with the Overlord Admin Account to allow
	/// users to preorder origin_of_shells through the `preorder_origin_of_shell()` function
	///
	/// Parameters:
	/// - `status`: Status to set CanPreorderOriginOfShells StorageValue
	fn set_preorder_origin_of_shells_status(status: bool) -> DispatchResult {
		<CanPreorderOriginOfShells<T>>::put(status);

		Self::deposit_event(Event::PreorderOriginOfShellsStatusChanged { status });

		Ok(())
	}

	/// Set status of last day of sale for origin of shells with the Overlord Admin Account to allow
	/// users to purchase any origin of shell
	///
	/// Parameters:
	/// - `status`: Status to set LastDayOfSale StorageValue
	fn set_last_day_of_sale_status(status: bool) -> DispatchResult {
		<LastDayOfSale<T>>::put(status);

		Self::deposit_event(Event::PreorderOriginOfShellsStatusChanged { status });

		Ok(())
	}

	/// Set initial OriginOfShellInventory values in the StorageDoubleMap. Key1 will be of
	/// OriginOfShellType and Key2 will be the RaceType and the Value will be NftSaleInfo struct
	/// containing the information for the NFT sale. Initial config will look as follows:
	/// `<Legendary>,<RaceType> => NftSaleInfo { race_count: 0, career_count: 0,
	/// race_for_sale_count: 1, race_giveaway_count: 0, race_reserved_count: 1 }`
	/// `<Magic>,<RaceType> => NftSaleInfo { race_count: 0, career_count: 0, race_for_sale_count:
	/// 15, race_giveaway_count: 0, race_reserved_count: 5 }`
	/// `<Prime>,<RaceType> => NftSaleInfo { race_count: 0, career_count: 0, race_for_sale_count:
	/// 1250, race_giveaway_count: 50, race_reserved_count: 0 }`
	fn set_initial_origin_of_shell_inventory() -> DispatchResult {
		// 3 OriginOfShellType Prime, Magic & Legendary and 4 different RaceType Cyborg, AISpectre,
		// XGene & Pandroid
		ensure!(
			!IsOriginOfShellsInventorySet::<T>::get(),
			Error::<T>::OriginOfShellInventoryAlreadySet
		);
		let legendary_nft_sale_info = NftSaleInfo {
			race_count: 0,
			race_for_sale_count: 1,
			race_giveaway_count: 0,
			race_reserved_count: 1,
		};
		OriginOfShellsInventory::<T>::insert(
			OriginOfShellType::Legendary,
			RaceType::AISpectre,
			legendary_nft_sale_info,
		);
		OriginOfShellsInventory::<T>::insert(
			OriginOfShellType::Legendary,
			RaceType::Cyborg,
			legendary_nft_sale_info,
		);
		OriginOfShellsInventory::<T>::insert(
			OriginOfShellType::Legendary,
			RaceType::Pandroid,
			legendary_nft_sale_info,
		);
		OriginOfShellsInventory::<T>::insert(
			OriginOfShellType::Legendary,
			RaceType::XGene,
			legendary_nft_sale_info,
		);
		let magic_nft_sale_info = NftSaleInfo {
			race_count: 0,
			race_for_sale_count: 10,
			race_giveaway_count: 0,
			race_reserved_count: 10,
		};
		OriginOfShellsInventory::<T>::insert(
			OriginOfShellType::Magic,
			RaceType::AISpectre,
			magic_nft_sale_info,
		);
		OriginOfShellsInventory::<T>::insert(
			OriginOfShellType::Magic,
			RaceType::Cyborg,
			magic_nft_sale_info,
		);
		OriginOfShellsInventory::<T>::insert(
			OriginOfShellType::Magic,
			RaceType::Pandroid,
			magic_nft_sale_info,
		);
		OriginOfShellsInventory::<T>::insert(
			OriginOfShellType::Magic,
			RaceType::XGene,
			magic_nft_sale_info,
		);
		let prime_nft_sale_info = NftSaleInfo {
			race_count: 0,
			race_for_sale_count: 1250,
			race_giveaway_count: 0,
			race_reserved_count: 0,
		};
		OriginOfShellsInventory::<T>::insert(
			OriginOfShellType::Prime,
			RaceType::AISpectre,
			prime_nft_sale_info,
		);
		OriginOfShellsInventory::<T>::insert(
			OriginOfShellType::Prime,
			RaceType::Cyborg,
			prime_nft_sale_info,
		);
		OriginOfShellsInventory::<T>::insert(
			OriginOfShellType::Prime,
			RaceType::Pandroid,
			prime_nft_sale_info,
		);
		OriginOfShellsInventory::<T>::insert(
			OriginOfShellType::Prime,
			RaceType::XGene,
			prime_nft_sale_info,
		);
		// Set IsOriginOfShellsInventorySet to true
		IsOriginOfShellsInventorySet::<T>::put(true);

		Ok(())
	}

	/// Update the NftSaleInfo for a given OriginOfShellType and RaceType
	///
	/// Parameters:
	/// - `origin_of_shell_type`: OriginOfShellType to update in OriginOfShellInventory
	/// - `race`: RaceType to update in OriginOfShellInventory
	/// - `for_sale_count`: count to increment the for sale count
	/// - `giveaway_count`: count to increment the race giveaway count
	fn update_nft_sale_info(
		origin_of_shell_type: OriginOfShellType,
		race: RaceType,
		for_sale_count: u32,
		giveaway_count: u32,
	) {
		OriginOfShellsInventory::<T>::mutate(origin_of_shell_type, race, |nft_sale_info| {
			if let Some(nft_sale_info) = nft_sale_info {
				nft_sale_info.race_for_sale_count =
					nft_sale_info.race_for_sale_count.saturating_add(for_sale_count);
				nft_sale_info.race_giveaway_count =
					nft_sale_info.race_giveaway_count.saturating_add(giveaway_count);
			}
		});
	}

	/// Mint a Spirit NFT helper function that will mint a Spirit NFT to new owner
	///
	/// Parameters:
	/// - `overlord`: Overlord account owns the NFT collection and will mint the NFT and freeze it
	/// - `sender`: New owner of the Spirit NFT
	fn do_mint_spirit_nft(overlord: T::AccountId, sender: T::AccountId) -> DispatchResult {
		// Has Spirit Collection been set
		let spirit_collection_id = Self::get_spirit_collection_id()?;
		// Check if sender already claimed a spirit
		ensure!(
			!Self::owns_nft_in_collection(&sender, spirit_collection_id),
			Error::<T>::SpiritAlreadyClaimed
		);
		// Empty metadata
		let metadata = Self::get_empty_metadata();
		// Get NFT ID to be minted
		let spirit_nft_id = pallet_rmrk_core::NextNftId::<T>::get(spirit_collection_id);
		// Mint new Spirit and transfer to sender
		pallet_rmrk_core::Pallet::<T>::mint_nft(
			Origin::<T>::Signed(overlord.clone()).into(),
			sender.clone(),
			spirit_collection_id,
			None,
			None,
			metadata,
		)?;
		// Freeze NFT so it cannot be transferred
		pallet_uniques::Pallet::<T>::freeze(
			Origin::<T>::Signed(overlord).into(),
			spirit_collection_id,
			spirit_nft_id,
		)?;

		Self::deposit_event(Event::SpiritClaimed {
			owner: sender,
			collection_id: spirit_collection_id,
			nft_id: spirit_nft_id,
		});

		Ok(())
	}

	/// Mint an Origin of Shell NFT helper function that will take in the shell type, race and
	/// career to mint the NFT to new owner
	///
	/// Parameters:
	/// - `overlord`
	/// - `sender`
	/// - `origin_of_shell_type`
	/// - `race`
	/// - `career`
	fn do_mint_origin_of_shell_nft(
		overlord: T::AccountId,
		sender: T::AccountId,
		origin_of_shell_type: OriginOfShellType,
		race: RaceType,
		career: CareerType,
		price: BalanceOf<T>,
		check_owned_origin_of_shell: bool,
	) -> DispatchResult {
		// Has Spirit Collection been set
		let spirit_collection_id = Self::get_spirit_collection_id()?;
		ensure!(
			Self::owns_nft_in_collection(&sender, spirit_collection_id),
			Error::<T>::MustOwnSpiritToPurchase,
		);
		// Ensure origin_of_shell collection is set
		let origin_of_shell_collection_id = Self::get_origin_of_shell_collection_id()?;
		// If check_owned_origin_of_shell then check if account owns an origin of shell
		if check_owned_origin_of_shell {
			ensure!(
				!Self::owns_nft_in_collection(&sender, origin_of_shell_collection_id),
				Error::<T>::OriginOfShellAlreadyPurchased
			);
		}
		// Get next NFT ID
		let nft_id = pallet_rmrk_core::NextNftId::<T>::get(origin_of_shell_collection_id);
		// Empty metadata
		let metadata = Self::get_empty_metadata();
		// Check if race and career types have mints left
		Self::has_race_type_left(&origin_of_shell_type, &race)?;
		// Transfer the amount for the rare Origin of Shell NFT then mint the origin_of_shell
		<T as pallet::Config>::Currency::transfer(
			&sender,
			&overlord,
			price,
			ExistenceRequirement::KeepAlive,
		)?;
		// Mint Origin of Shell and transfer Origin of Shell to new owner
		pallet_rmrk_core::Pallet::<T>::mint_nft(
			Origin::<T>::Signed(overlord.clone()).into(),
			sender.clone(),
			origin_of_shell_collection_id,
			None,
			None,
			metadata,
		)?;
		// Set Origin of Shell Type, Race and Career attributes for NFT
		Self::set_nft_attributes(
			origin_of_shell_collection_id,
			nft_id,
			origin_of_shell_type,
			race,
			career,
		)?;
		// Update storage
		Self::decrement_race_type_left(origin_of_shell_type, race)?;
		Self::increment_race_type(origin_of_shell_type, race)?;
		Self::increment_career_type(career);

		// Freeze NFT so it cannot be transferred
		pallet_uniques::Pallet::<T>::freeze(
			Origin::<T>::Signed(overlord).into(),
			origin_of_shell_collection_id,
			nft_id,
		)?;

		Self::deposit_event(Event::OriginOfShellMinted {
			origin_of_shell_type,
			collection_id: origin_of_shell_collection_id,
			nft_id,
			owner: sender,
			race,
			career,
		});

		Ok(())
	}

	/// Set the attributes for Origin of Shell or Shell NFT's type, race and career.
	///
	/// Parameters:
	/// - `collection_id`: Collection id of the Origin of Shell or Shell NFT
	/// - `nft_id`: NFT id of the Origin of Shell or Shell NFT
	/// - `origin_of_shell_type`: Origin of Shell or Shell type for the NFT
	/// - `race`: Race attribute to set for the Origin of Shell or Shell NFT
	/// - `career`: Career attribute to set for the Origin of Shell or Shell NFT
	pub(crate) fn set_nft_attributes(
		collection_id: CollectionId,
		nft_id: NftId,
		origin_of_shell_type: OriginOfShellType,
		race: RaceType,
		career: CareerType,
	) -> DispatchResult {
		let overlord = Self::overlord()?;

		let origin_of_shell_type_key: BoundedVec<u8, T::KeyLimit> =
			Self::to_boundedvec_key("origin_of_shell_type")?;
		let origin_of_shell_type_value = origin_of_shell_type
			.encode()
			.try_into()
			.expect("[origin_of_shell_type] should not fail");

		let race_key: BoundedVec<u8, T::KeyLimit> = Self::to_boundedvec_key("race")?;
		let race_value = race.encode().try_into().expect("[race] should not fail");

		let career_key = Self::to_boundedvec_key("career")?;
		let career_value = career.encode().try_into().expect("[career] should not fail");

		// Set Origin of Shell Type
		pallet_uniques::Pallet::<T>::set_attribute(
			Origin::<T>::Signed(overlord.clone()).into(),
			collection_id,
			Some(nft_id),
			origin_of_shell_type_key,
			origin_of_shell_type_value,
		)?;
		// Set Race
		pallet_uniques::Pallet::<T>::set_attribute(
			Origin::<T>::Signed(overlord.clone()).into(),
			collection_id,
			Some(nft_id),
			race_key,
			race_value,
		)?;
		// Set Career
		pallet_uniques::Pallet::<T>::set_attribute(
			Origin::<T>::Signed(overlord).into(),
			collection_id,
			Some(nft_id),
			career_key,
			career_value,
		)?;

		Ok(())
	}

	/// Decrement CareerType count for the `career`
	///
	/// Parameters:
	/// - `career`: The Career to increment count
	fn decrement_career_type(career: CareerType) {
		CareerTypeCount::<T>::mutate(career, |career_count| {
			*career_count -= 1;
			*career_count
		});
	}

	/// Increment RaceType count for the `race`
	///
	/// Parameters:
	/// - `origin_of_shell_type`: Origin of Shell type
	/// - `race`: The Career to increment count
	fn increment_race_type(
		origin_of_shell_type: OriginOfShellType,
		race: RaceType,
	) -> DispatchResult {
		OriginOfShellsInventory::<T>::try_mutate_exists(
			origin_of_shell_type,
			race,
			|nft_sale_info| -> DispatchResult {
				if let Some(nft_sale_info) = nft_sale_info {
					nft_sale_info.race_count += 1;
				}
				Ok(())
			},
		)?;

		Ok(())
	}

	/// Increment CareerType count for the `career`
	///
	/// Parameters:
	/// - `career`: The Career to increment count
	fn increment_career_type(career: CareerType) {
		CareerTypeCount::<T>::mutate(career, |career_count| {
			*career_count += 1;
			*career_count
		});
	}

	/// Decrement RaceType count for the `race`
	///
	/// Parameters:
	/// - `race`: The Race to increment count
	fn decrement_race_type_left(
		origin_of_shell_type: OriginOfShellType,
		race: RaceType,
	) -> DispatchResult {
		OriginOfShellsInventory::<T>::try_mutate_exists(
			origin_of_shell_type,
			race,
			|nft_sale_info| -> DispatchResult {
				if let Some(nft_sale_info) = nft_sale_info {
					nft_sale_info.race_for_sale_count.saturating_sub(1);
				}
				Ok(())
			},
		)?;

		Ok(())
	}

	/// Verify if the chosen Race has reached the max limit
	///
	/// Parameters:
	/// - `race`: The Race to check
	fn has_race_type_left(
		origin_of_shell_type: &OriginOfShellType,
		race: &RaceType,
	) -> DispatchResult {
		if let Some(nft_sale_info) = OriginOfShellsInventory::<T>::get(origin_of_shell_type, race) {
			ensure!(nft_sale_info.race_for_sale_count > 0, Error::<T>::RaceMintMaxReached);
		} else {
			return Err(Error::<T>::OriginOfShellInventoryCorrupted.into())
		}

		Ok(())
	}

	/// Helper function to get collection id origin of shell collection
	fn get_origin_of_shell_collection_id() -> Result<CollectionId, Error<T>> {
		let origin_of_shell_collection_id = OriginOfShellCollectionId::<T>::get()
			.ok_or(Error::<T>::OriginOfShellCollectionNotSet)?;
		Ok(origin_of_shell_collection_id)
	}

	/// Helper function to get collection id spirit collection
	fn get_spirit_collection_id() -> Result<CollectionId, Error<T>> {
		let spirit_collection_id =
			SpiritCollectionId::<T>::get().ok_or(Error::<T>::SpiritCollectionNotSet)?;
		Ok(spirit_collection_id)
	}

	/// Helper function to check if owner has a NFT within a collection
	///
	/// Parameters:
	/// `sender`: reference to the account id to check
	/// `collection_id`: Collection id to check if sender owns a NFT in the collection
	/// `error`: Error type to throw if there is an error detected
	pub fn owns_nft_in_collection(sender: &T::AccountId, collection_id: CollectionId) -> bool {
		pallet_uniques::Pallet::<T>::owned_in_class(&collection_id, sender).count() > 0
	}

	pub fn to_boundedvec_key(name: &str) -> Result<BoundedVec<u8, T::KeyLimit>, Error<T>> {
		name.as_bytes().to_vec().try_into().map_err(|_| Error::<T>::KeyTooLong)
	}

	/// Helper function to get empty metadata boundedvec
	pub(crate) fn get_empty_metadata() -> BoundedVec<u8, T::StringLimit> {
		Default::default()
	}
}
