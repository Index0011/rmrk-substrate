# Phala World Scripts
## [0] Initial Configuration (Spirit Claim)
By default, the `init.js` script will assign the `root` account as `ALICE`, the `user` account as `BOB` and `overlord` account set as `BOB`. Running the `init.js` script performs the following:
> Execute the init.js script:
>```shell
>node ./init.js
>```
### Set Overlord account using sudo
Sudo transaction from `root` account aka `ALICE` that will call the privileged transaction `setOverlord` to set the `overlord` account to `BOB`
```javascript
await api.tx.sudo.sudo(
    api.tx.pwNftSale.setOverlord(overlord.address)
).signAndSend(root, {nonce: -1});
await sleep(6000);
```

### Initialize Phala World Clock
Overlord account then starts the Phala World Clock to initialize the `ZeroDay` Timestamp in transaction `initializeWorldClock()` to signify the official beginning of Phala World
```javascript
await api.tx.pwNftSale.initializeWorldClock()
    .signAndSend(overlord, {nonce: -1});
```

### Enable Spirit Claims
Overlord account will then enable the Spirit Claims for accounts to claim a non-transferable NFT representing their Spirit. This is done by calling transaction `setStatusType` with parameters `bool, StatusType` where `StatusType` is an enum containing `ClaimSpirits, PurchaseRareOriginOfShells, PurchasePrimeOriginOfShells, PreorderOriginOfShells, LastDayOfSale`. Here we use `ClaimSpirits` and set it to `true`.
```javascript
await api.tx.pwNftSale.setStatusType(true, 'ClaimSpirits')
    .signAndSend(overlord, {nonce: -1});
```

### Create Spirit & Origin of Shell Collection IDs
Overlord creates the Spirit & Origin of Shell Collection IDs for the two NFT collections then sets the Collection IDs for each in Storage
```javascript
// mint spirits NFTs with overlord
// collection 0: spirits
await api.tx.rmrkCore.createCollection(
    '0x',
    null,
    'PWSPRT'
).signAndSend(overlord, {nonce: -1});
// set the spirits collection id
await api.tx.pwNftSale.setSpiritCollectionId(
    0
).signAndSend(overlord, {nonce: -1});
// collection 1: origin of shells
await api.tx.rmrkCore.createCollection(
    '0x',
    null,
    'PWOAS'
).signAndSend(overlord, {nonce: -1});
// set the origin of shell collection id
await api.tx.pwNftSale.setOriginOfShellCollectionId(
    1
).signAndSend(overlord, {nonce: -1});
```

### Initialize the Inventory counts
In the `init.js` script there is a transaction that will set the starting inventory counts for the initial sales until the preorder phase. This script will populate the StorageDoubleMap called `originOfShellsInventory`.
```javascript
await api.tx.pwNftSale.initOriginOfShellTypeCounts().signAndSend(overlord, {nonce: -1});
```

### Generate RedeemSpirit and BuyPrimeOriginOfShells Signatures
To avoid accounts transacting with the runtime directly via polkadot.js or scripts, the metadata is signed by the `overlord` account. This will allow for the backend to verify the claim and proceed with minting of a given NFT. Here is an example of signing the `OverlordMessage` with the account address and a purpose enum `Purpose` with values of `RedeemSpirit` or `BuyPrimeOriginOfShells`.
```javascript
// RedeemSpirit
const purpose = api.createType('Purpose', 'RedeemSpirit');
const overlordMessage = api.createType('OverlordMessage', {'account': ferdie.address, 'purpose': purpose});
const overlordSig = overlord.sign(overlordMessage.toU8a());
```

### Claim a Spirit
This is an example of generating the `Signature` for a Spirit NFT by adding a prefix `"RS"` to `ferdie.address` then sign the encoding with `overlord` account then using the `ferdie` account to claim the spirit.
```javascript
// Mint a Spirit with at lest 10 PHA
await api.tx.pwNftSale.claimSpirit().signAndSend(ferdie);
// Redeem a Spirit with a valid Signature
await api.tx.pwNftSale.redeemSpirit(overlordSig).signAndSend(ferdie);
```

### Status Types
There are a few `StatusType` to note with different meanings. These status types can be changed by utilizing the `Overlord` admin account to execute a transaction called `setStatusType(bool, StatusType)`.
Here is an example of enabling Spirit claims:
```javascript
await api.tx.pwNftSale.setStatusType(true, 'ClaimSpirits')
    .signAndSend(overlord);
```
Next we will go into some details on the different `StatusType`:
- `ClaimSpirits`: Determines the status of the current Spirit Claim process. When this is set, there is a `bool` in storage to signify if Spirits can be claimed at the given moment.
- `PurchaseRareOriginOfShells`: Determines the status of Rare (Legendary or Magic) Origin of Shells being able to be purchased. A `bool` in storage represents the current status.
- `PurchasePrimeOriginOfShells`: Determines the status of Whitelist accounts being able to purchase a Prime Origin of Shell. This is mapped to a `bool` in storage to determine if Whitelist users can purchase.
- `PreorderOriginOfShells`: Determines the status of the Preorders for a chance to mint an Origin of Shell. A `bool` in storage represents the current status.
- `LastDayOfSale`: Determines if the last day of Origin of Shell sales is true and allows for unlimited purchases of Origin of Shell without previous restrictions based on available quantity.

## [1] Enable Rare Origin of Shells Sale
Next, there will be a phase that allows accounts to purchase a rare Origin of Shell NFT. First the `overlord` account will enable the `StatusType` `PurchaseRareOriginOfShells`.
```javascript
await api.tx.pwNftSale.setStatusType(true, 'PurchaseRareOriginOfShells')
    .signAndSend(overlord);
```
Here is an example of a user executing a transaction called `buyRareOriginOfShell` to purchase a rare Origin of Shell NFT. The Parameters can be as follows:
- `OriginOfShellType`: Origin of Shell Type and in this case the 2 acceptable values are `'Legendary'` or `'Magic'`.
- `RaceType`: A pick of any of the 4 Races `'Cyborg'`, `'AISpectre'`, `'Pandroid'`, `'XGene'`.
- `CareerType`: A pick of any of the 5 Careers `'HardwareDruid'`, `'RoboWarrior'`, `'TradeNegotiator'`, `'HackerWizard'`, `'Web3Monk'`.
```javascript
// Purchase rare Origin of Shell
await api.tx.pwNftSale.buyRareOriginOfShell('Legendary', 'Cyborg', 'HackerWizard', nftSignedMetadata)
    .signAndSend(user);
```

## [2] Enable Whitelist Sale
After the rare Origin of Shell purchases, we will then move to the Whitelist purchases. This will involve another validation effort by the `overlord` account signing some metadata along with the whitelisted account ID. This will be a valid `Signature` and will be passed into the transaction called `buyPrimeOriginOfShell`. First, enable the `StatusType` `PurchasePrimeOriginOfShells` before proceeding.
```javascript
await api.tx.pwNftSale.setStatusType(true, 'PurchasePrimeOriginOfShells')
    .signAndSend(overlord);
```
Here is an example of creating a `Signature` for the `ferdie` account where a `purpose` of `BuyPrimeOriginOfShells` is added to the account address. This is what `ferdie` will use to pass into the `buyPrimeOriginOfShell` function. 
```javascript
// BuyPrimeOriginOfShells
const purpose = api.createType('Purpose', 'BuyPrimeOriginOfShells');
const overlordMessage = api.createType('OverlordMessage', {'account': ferdie.address, 'purpose': purpose});
const overlordSig = overlord.sign(overlordMessage.toU8a());
```
This will enable `ferdie` to call `PurchasePrimeOriginOfShells` and here is an explanation of the valid parameters:
- `sr25519::Signature`: a `signature` of the `&OverlordMessage` by the `overlord` account to validate the whitelist claim by a given account.
- `OverlordMessage`: a struct message that is encoded & signed by `Overlord` account that holds the `account` and `purpose` of the message
- `RaceType`: A pick of any of the 4 Races `'Cyborg'`, `'AISpectre'`, `'Pandroid'`, `'XGene'`.
- `CareerType`: A pick of any of the 5 Careers `'HardwareDruid'`, `'RoboWarrior'`, `'TradeNegotiator'`, `'HackerWizard'`, `'Web3Monk'`.
```javascript
await api.tx.pwNftSale.buyPrimeOriginOfShell(overlordSig, 'Cyborg', 'HackerWizard')
    .signAndSend(ferdie);
```

## [3] Enable Preorders of Origin of Shell
Preorders will be similar in simplicity like the rare Origin of Shell purchases. First, disable the `StatusType` for `PurchasePrimeOriginOfShells` then we will calculate the remaining `Prime` Origin of Shell NFTs and add an addition 900 Prime Origin of Shell NFT to inventory. Lastly, enable the `StatusType` `PreorderOriginOfShells` to allow for users to begin preordering tickets for the Non-WL drawing.
```javascript
// Disable Whitelist purchases
await api.tx.pwNftSale.setStatusType(false, 'PurchasePrimeOriginOfShells')
    .signAndSend(overlord);
// Update inventory with 900 extra `Prime` Origin of Shell NFTs to sell and 50 per species to giveaway
await api.tx.pwNftSale.updateOriginOfShellTypeCounts('Prime', 900, 50).signAndSend(overlord);
await api.tx.pwNftSale.setStatusType(true, 'PreorderOriginOfShells')
    .signAndSend(overlord);
```
Here is an example of a Preorder transaction `preorderOriginOfShell`:
```javascript
await api.tx.pwNftSale.preorderOriginOfShell('Pandroid', 'HackerWizard')
    .signAndSend(ferdie);
```
### After Preorders Are Finalized
After the preorders are finalized, disable the `StatusType` `PreorderOriginOfShells`. Then run a query on all the Preorders in storage.
```javascript
await api.tx.pwNftSale.setStatusType(false, 'PreorderOriginOfShells')
    .signAndSend(overlord);
// Query all preorders
const preorderIndex = await api.query.pwNftSale.preorderIndex();
console.log(`Current preorder index: ${preorderIndex}`);
const preorderKeys = await api.query.pwNftSale.preorders.entries();
preorderKeys
    .map(([key, value]) =>
        [key.args[0].toNumber(), value.toHuman()]
    ).forEach(([preorderId, preorderInfo]) => {
    console.log({
        preorderId,
        preorderInfo,
    })
})
```
Next, create a script to randomly select chosen Preorder IDs. Then create an array of chosen Preorder IDs to automatically mint the preorders to the owner of the preorders. 
- `PreorderId`: A number ID mapped to the preorder.
- `Preorders`: A Vec of chosen preorders
```javascript
const chosenPreorders = api.createType('Vec<u32>', [0, 1, 2, 4, 10, 6, 12, 11]);
await api.tx.pwNftSale.mintChosenPreorders(chosenPreorders)
    .signAndSend(overlord);
```

Lastly, create take the not chosen Preorder IDs and refund the account owners that reserved payment for the preorder.
- `PreorderId`: A number ID mapped to the preorder.
- `Preorders`: A Vec of chosen preorders
```javascript
const notChosenPreorders = api.createType('Vec<u32>', [7, 3, 5, 8, 9, 13]);
await api.tx.pwNftSale.refundNotChosenPreorders(notChosenPreorders)
    .signAndSend(overlord);
```

## [4] Enable Last Day of Sale
Enable the last day of sale for unlimited purchases for all 3 levels of the remaining Origin of Shell supply.
```javascript
await api.tx.pwNftSale.setStatusType(true, 'LastDayOfSale')
    .signAndSend(overlord);
```

## [5] Initiate the Incubation Phase
Start the incubation phase so Origin of Shells can start feeding other Origin of Shells
```javascript
await api.tx.pwIncubation.setCanStartIncubationStatus(true)
    .signAndSend(overlord);
```

## [6] Feed Other Origin of Shells
Here is an example of accounts feeding other Origin of Shells.
```javascript
console.log(`Sending food among accounts...`);
await api.tx.pwIncubation.feedOriginOfShell(1, 0).signAndSend(alice, { nonce: nonceAlice++ });
await api.tx.pwIncubation.feedOriginOfShell(1, 0).signAndSend(bob, { nonce: nonceBob++ });
await api.tx.pwIncubation.feedOriginOfShell(1, 1).signAndSend(charlie, { nonce: nonceCharlie++ });
await api.tx.pwIncubation.feedOriginOfShell(1, 2).signAndSend(david, { nonce: nonceDavid++ });
await api.tx.pwIncubation.feedOriginOfShell(1, 3).signAndSend(eve, { nonce: nonceEve++ });
await api.tx.pwIncubation.feedOriginOfShell(1, 4).signAndSend(ferdie, { nonce: nonceFerdie++ });
await waitTxAccepted(alice.address, nonceAlice - 1);
await api.tx.pwIncubation.feedOriginOfShell(1, 5).signAndSend(alice, { nonce: nonceAlice++ });
await api.tx.pwIncubation.feedOriginOfShell(1, 1).signAndSend(bob, { nonce: nonceBob++ });
await api.tx.pwIncubation.feedOriginOfShell(1, 3).signAndSend(charlie, { nonce: nonceCharlie++ });
await api.tx.pwIncubation.feedOriginOfShell(1, 8).signAndSend(david, { nonce: nonceDavid++ });
await api.tx.pwIncubation.feedOriginOfShell(1, 2).signAndSend(eve, { nonce: nonceEve++ });
await api.tx.pwIncubation.feedOriginOfShell(1, 10).signAndSend(ferdie, { nonce: nonceFerdie++ });
console.log(`Sending food among accounts...Done`);
```
Here is a query to get the stats per era of the times an Origin of Shell was fed.
```javascript
const currentEra = await api.query.pwNftSale.era();
console.log(`Current Era: ${currentEra}`);
// Times fed in era 0 for the [collectionId, nftId], era
const originOfShellFoodStats = await api.query.pwIncubation.originOfShellFoodStats.entries();
originOfShellFoodStats
    .map(([key, value]) =>
        [key.args[0].toHuman(), key.args[1].toNumber(), value.toNumber()]
    ).forEach(([collectionIdNftId, era, value]) => {
    console.log({
        collectionIdNftId,
        era,
        value,
    })
})
```
Results
```shell
Current Era: 0
{ collectionIdNftId: [ '1', '2' ], era: 0, value: 2 }
{ collectionIdNftId: [ '1', '3' ], era: 0, value: 2 }
{ collectionIdNftId: [ '1', '5' ], era: 0, value: 1 }
{ collectionIdNftId: [ '1', '8' ], era: 0, value: 1 }
{ collectionIdNftId: [ '1', '0' ], era: 0, value: 2 }
{ collectionIdNftId: [ '1', '1' ], era: 0, value: 2 }
{ collectionIdNftId: [ '1', '10' ], era: 0, value: 1 }
{ collectionIdNftId: [ '1', '4' ], era: 0, value: 1 }
```

## [7] Reduce Top 10 Fed in Era
This is a privileged function for the Overlord account to take the top 10 fed Origin of Shells of the Era.
```javascript
const originOfShells = api.createType('Vec<((u32,u32), u64)>', [[[1, 1], 10800], [[1,0], 7200], [[1, 3], 3600], [[1,2], 2400], [[1, 0], 2400], [[1, 4], 1400], [[1, 5], 1400], [[1, 8], 1400], [[1, 10], 1400]]);
        await api.tx.pwIncubation.updateIncubationTime(originOfShells).signAndSend(overlord);
```
Here is a query with results of the Origin of Shell Hatch Times
```javascript
const currentEra = await api.query.pwNftSale.era();
console.log(`Current Era: ${currentEra}`);
// hatchTimes for the Collection ID
const hatchTimes = await api.query.pwIncubation.hatchTimes.entries(1);
hatchTimes
    .map(([key, value]) =>
        [key.args[0].toString(), key.args[1].toNumber(), value.toHuman()]
    ).forEach(([collectionId, nftId, timestamp]) => {
    console.log({
        collectionId,
        nftId,
        timestamp,
    })
})
```
Results
```shell
Current Era: 0
{ collectionId: '1', nftId: 0, timestamp: '1,654,375,860' }
{ collectionId: '1', nftId: 10, timestamp: '1,654,384,060' }
{ collectionId: '1', nftId: 4, timestamp: '1,654,384,060' }
{ collectionId: '1', nftId: 2, timestamp: '1,654,383,060' }
{ collectionId: '1', nftId: 5, timestamp: '1,654,384,060' }
{ collectionId: '1', nftId: 8, timestamp: '1,654,384,060' }
{ collectionId: '1', nftId: 1, timestamp: '1,654,374,660' }
{ collectionId: '1', nftId: 3, timestamp: '1,654,381,860' }

```
## [8] Hatch Origin of Shells to Shell NFT
This is a privileged function for the Overlord account that will burn the Origin of Shell NFT and mint the Shell NFT for the account.
```javascript
const metadata = api.createType('BoundedVec<u8, <T as pallet_uniques::Config>::StringLimit>', "https://a2l45nwayr2ij5g7aypdjw7sq2i4eceuowszzeqjw54fr3wu.arweave.net/BpfOtsDEdIT03wYeNNvy-hpHCCJR1pZySCbd4WO-7Uw/");
        await api.tx.pwIncubation.hatchOriginOfShell(ferdie.address, 1, 0, metadata).signAndSend(overlord);
```