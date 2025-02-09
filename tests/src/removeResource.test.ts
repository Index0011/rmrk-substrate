import { expect } from "chai";
import privateKey from "./substrate/privateKey";
import {
  executeTransaction,
  getApiConnection,
} from "./substrate/substrate-api";
import { getNft, NftIdTuple } from "./util/fetch";
import { expectTxFailure } from "./util/helpers";
import {
  acceptNft,
  acceptResourceRemoval,
  addNftBasicResource,
  createBase,
  createCollection,
  mintNft,
  removeNftResource,
  sendNft,
} from "./util/tx";

describe("Integration test: remove nft resource", () => {
  let api: any;
  let ss58Format: string;
  before(async () => {
    api = await getApiConnection();
    ss58Format = api.registry.getChainProperties()!.toJSON().ss58Format;
  });

  const Alice = "//Alice";
  const Bob = "//Bob";
  const metadata = "test-basic-metadata";

  it("deleting a resource directly by the NFT owner", async () => {
    const collectionIdAlice = await createCollection(
      api,
      Alice,
      "test-metadata",
      null,
      "test-symbol"
    );

    const nftAlice = await mintNft(
      api,
      0,
      Alice,
      Alice,
      collectionIdAlice,
      "nft-metadata"
    );

    const resourceId = await addNftBasicResource(
      api,
      0,
      Alice,
      "added",
      collectionIdAlice,
      nftAlice,
      metadata
    );

    await removeNftResource(
      api,
      "removed",
      Alice,
      collectionIdAlice,
      nftAlice,
      resourceId
    );
  });

  it("deleting resources indirectly by the NFT owner", async () => {
    const collectionIdAlice = await createCollection(
      api,
      Alice,
      "test-metadata",
      null,
      "test-symbol"
    );

    const parentNftId = await mintNft(
      api,
      0,
      Alice,
      Alice,
      collectionIdAlice,
      "parent-nft-metadata"
    );
    const childNftId = await mintNft(
      api,
      1,
      Alice,
      Alice,
      collectionIdAlice,
      "child-nft-metadata"
    );

    const resourceId = await addNftBasicResource(
      api,
      0,
      Alice,
      "added",
      collectionIdAlice,
      childNftId,
      metadata
    );

    const newOwnerNFT: NftIdTuple = [collectionIdAlice, parentNftId];

    await sendNft(
      api,
      "sent",
      Alice,
      collectionIdAlice,
      childNftId,
      newOwnerNFT
    );

    await removeNftResource(
      api,
      "removed",
      Alice,
      collectionIdAlice,
      childNftId,
      resourceId
    );
  });

  it("deleting a resource by the collection owner", async () => {
    const collectionIdAlice = await createCollection(
      api,
      Alice,
      "test-metadata",
      null,
      "test-symbol"
    );

    const nftBob = await mintNft(
      api,
      0,
      Alice,
      Bob,
      collectionIdAlice,
      "nft-metadata"
    );

    const resourceId = await addNftBasicResource(
      api,
      0,
      Alice,
      "pending",
      collectionIdAlice,
      nftBob,
      metadata
    );

    await removeNftResource(
      api,
      "pending",
      Alice,
      collectionIdAlice,
      nftBob,
      resourceId
    );
    await acceptResourceRemoval(
      api,
      Bob,
      collectionIdAlice,
      nftBob,
      resourceId
    );
  });

  it("deleting a resource in a nested NFT by the collection owner", async () => {
    const collectionIdAlice = await createCollection(
      api,
      Alice,
      "test-metadata",
      null,
      "test-symbol"
    );

    const parentNftId = await mintNft(
      api,
      0,
      Alice,
      Bob,
      collectionIdAlice,
      "parent-nft-metadata"
    );
    const childNftId = await mintNft(
      api,
      1,
      Alice,
      Bob,
      collectionIdAlice,
      "child-nft-metadata"
    );

    const resourceId = await addNftBasicResource(
      api,
      0,
      Alice,
      "pending",
      collectionIdAlice,
      childNftId,
      metadata
    );

    const newOwnerNFT: NftIdTuple = [collectionIdAlice, parentNftId];

    await sendNft(
      api,
      "pending",
      Bob,
      collectionIdAlice,
      childNftId,
      newOwnerNFT
    );

    await removeNftResource(
      api,
      "pending",
      Alice,
      collectionIdAlice,
      childNftId,
      resourceId
    );
    await acceptResourceRemoval(
      api,
      Bob,
      collectionIdAlice,
      childNftId,
      resourceId
    );
  });

  it("[negative]: can't delete a resource in a non-existing collection", async () => {
    const collectionIdAlice = await createCollection(
      api,
      Alice,
      "test-metadata",
      null,
      "test-symbol"
    );

    const nftAlice = await mintNft(
      api,
      0,
      Alice,
      Alice,
      collectionIdAlice,
      "nft-metadata"
    );

    const resourceId = await addNftBasicResource(
      api,
      0,
      Alice,
      "added",
      collectionIdAlice,
      nftAlice,
      metadata
    );

    const tx = removeNftResource(
      api,
      "removed",
      Alice,
      0xffffffff,
      nftAlice,
      resourceId
    );
    await expectTxFailure(/rmrkCore\.CollectionUnknown/, tx); // FIXME: inappropriate error message (NoAvailableNftId)
  });

  it("[negative]: only collection owner can delete a resource", async () => {
    const collectionIdAlice = await createCollection(
      api,
      Alice,
      "test-metadata",
      null,
      "test-symbol"
    );

    const nftAlice = await mintNft(
      api,
      0,
      Alice,
      Alice,
      collectionIdAlice,
      "nft-metadata"
    );

    const resourceId = await addNftBasicResource(
      api,
      0,
      Alice,
      "added",
      collectionIdAlice,
      nftAlice,
      metadata
    );

    const tx = removeNftResource(
      api,
      "removed",
      Bob,
      collectionIdAlice,
      nftAlice,
      resourceId
    );
    await expectTxFailure(/rmrkCore\.NoPermission/, tx);
  });

  it("[negative]: cannot delete a resource that does not exist", async () => {
    const collectionIdAlice = await createCollection(
      api,
      Alice,
      "test-metadata",
      null,
      "test-symbol"
    );

    const nftAlice = await mintNft(
      api,
      0,
      Alice,
      Alice,
      collectionIdAlice,
      "nft-metadata"
    );

    const tx = removeNftResource(
      api,
      "removed",
      Alice,
      collectionIdAlice,
      nftAlice,
      127
    );
    await expectTxFailure(/rmrkCore\.ResourceDoesntExist/, tx);
  });

  it("[negative]: Cannot accept deleting resource without owner attempt do delete it", async () => {
    const collectionIdAlice = await createCollection(
      api,
      Alice,
      "test-metadata",
      null,
      "test-symbol"
    );

    const nftBob = await mintNft(
      api,
      0,
      Alice,
      Bob,
      collectionIdAlice,
      "nft-metadata"
    );

    const resourceId = await addNftBasicResource(
      api,
      0,
      Alice,
      "pending",
      collectionIdAlice,
      nftBob,
      metadata
    );

    const tx = acceptResourceRemoval(
      api,
      Bob,
      collectionIdAlice,
      nftBob,
      resourceId
    );
    await expectTxFailure(/rmrkCore\.ResourceNotPending/, tx);
  });

  it("[negative]: cannot confirm the deletion of a non-existing resource", async () => {
    const collectionIdAlice = await createCollection(
      api,
      Alice,
      "test-metadata",
      null,
      "test-symbol"
    );

    const nftBob = await mintNft(
      api,
      0,
      Alice,
      Bob,
      collectionIdAlice,
      "nft-metadata"
    );

    const tx = acceptResourceRemoval(api, Bob, collectionIdAlice, nftBob, 127);
    await expectTxFailure(/rmrkCore\.ResourceDoesntExist/, tx);
  });

  it("[negative]: Non-owner user cannot confirm the deletion of resource", async () => {
    const collectionIdAlice = await createCollection(
      api,
      Alice,
      "test-metadata",
      null,
      "test-symbol"
    );

    const nftAlice = await mintNft(
      api,
      0,
      Alice,
      Alice,
      collectionIdAlice,
      "nft-metadata"
    );

    const resourceId = await addNftBasicResource(
      api,
      0,
      Alice,
      "added",
      collectionIdAlice,
      nftAlice,
      metadata
    );

    const tx = acceptResourceRemoval(
      api,
      Bob,
      collectionIdAlice,
      nftAlice,
      resourceId
    );
    await expectTxFailure(/rmrkCore\.NoPermission/, tx);
  });

  after(() => {
    api.disconnect();
  });
});
