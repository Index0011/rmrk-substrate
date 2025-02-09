import { getApiConnection } from "./substrate/substrate-api";
import { NftIdTuple } from "./util/fetch";
import { expectTxFailure } from "./util/helpers";
import { createCollection, mintNft, sendNft, setNftProperty } from "./util/tx";

describe("integration test: set NFT property", () => {
  let api: any;
  before(async () => {
    api = await getApiConnection();
  });

  const alice = "//Alice";
  const bob = "//Bob";

  const createTestCollection = async (issuerUri: string) => {
    return await createCollection(
      api,
      issuerUri,
      "setprop-nft-collection-metadata",
      null,
      "setprop"
    );
  };

  it("set NFT property", async () => {
    const ownerAlice = alice;

    const collectionId = await createTestCollection(alice);
    const nftId = await mintNft(
      api,
      300,
      alice,
      ownerAlice,
      collectionId,
      "prop-nft"
    );

    await setNftProperty(
      api,
      alice,
      collectionId,
      nftId,
      "test-key",
      "test-key-value"
    );
    await setNftProperty(
      api,
      alice,
      collectionId,
      nftId,
      "test-key",
      "updated-key-value"
    );
    await setNftProperty(
      api,
      alice,
      collectionId,
      nftId,
      "second-test-key",
      "second-test-key-value"
    );
  });

  it("[negative] unable to set a property of non-existing NFT", async () => {
    const collectionId = 0;
    const maxNftId = 0xffffffff;

    const tx = setNftProperty(
      api,
      alice,
      collectionId,
      maxNftId,
      "test-key",
      "test-value"
    );

    await expectTxFailure(/rmrkCore\.NoAvailableNftId/, tx);
  });

  it("[negative] unable to set a property by not-an-owner", async () => {
    const ownerAlice = alice;

    const collectionId = await createTestCollection(alice);
    const nftId = await mintNft(
      api,
      301,
      alice,
      ownerAlice,
      collectionId,
      "prop-nft"
    );

    const tx = setNftProperty(
      api,
      bob,
      collectionId,
      nftId,
      "test-key",
      "test-key-value"
    );

    await expectTxFailure(/rmrkCore\.NoPermission/, tx);
  });

  it("set a property to nested NFT", async () => {
    const ownerAlice = alice;

    const collectionId = await createTestCollection(alice);
    const parentNftId = await mintNft(
      api,
      302,
      alice,
      ownerAlice,
      collectionId,
      "prop-parent-nft"
    );
    const childNftId = await mintNft(
      api,
      303,
      alice,
      ownerAlice,
      collectionId,
      "prop-child-nft"
    );

    const ownerNft: NftIdTuple = [collectionId, parentNftId];

    await sendNft(api, "sent", ownerAlice, collectionId, childNftId, ownerNft);

    await setNftProperty(
      api,
      alice,
      collectionId,
      childNftId,
      "test-key",
      "test-key-value"
    );
  });

  it("[negative] set a property to nested NFT (by not-root-owner)", async () => {
    const ownerAlice = alice;

    const collectionId = await createTestCollection(alice);
    const parentNftId = await mintNft(
      api,
      0,
      alice,
      ownerAlice,
      collectionId,
      "prop-parent-nft"
    );
    const childNftId = await mintNft(
      api,
      304,
      alice,
      ownerAlice,
      collectionId,
      "prop-child-nft"
    );

    const ownerNft: NftIdTuple = [collectionId, parentNftId];

    await sendNft(api, "sent", ownerAlice, collectionId, childNftId, ownerNft);

    const tx = setNftProperty(
      api,
      bob,
      collectionId,
      childNftId,
      "test-key",
      "test-key-value"
    );

    await expectTxFailure(/rmrkCore\.NoPermission/, tx);
  });

  after(() => {
    api.disconnect();
  });
});
