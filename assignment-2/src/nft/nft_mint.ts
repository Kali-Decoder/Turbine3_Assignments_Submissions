import "dotenv/config";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import {
    createSignerFromKeypair,
    generateSigner,
    signerIdentity,
} from "@metaplex-foundation/umi";
import { create, mplCore, ruleSet } from "@metaplex-foundation/mpl-core";
import { base58 } from "@metaplex-foundation/umi/serializers";

import wallet from "../../devnet-wallet.json";

const umi = createUmi(process.env.SOLANA_RPC_URL ?? "https://api.devnet.solana.com");

const keypair = umi.eddsa.createKeypairFromSecretKey(new Uint8Array(wallet));
const signer = createSignerFromKeypair(umi, keypair);

umi.use(signerIdentity(signer));
umi.use(mplCore());

(async () => {
    try {
        // paste the metadata URI printed by nft_metadata.ts
        const metadataUri = "https://gateway.irys.xyz/By4WDd6m9UEMmLgygYgpkUQMtEqnCKjGhqbusb5NB1ta";
        const asset = generateSigner(umi);

        const tx = await create(umi, {
            asset,
            name: "Kali-Decoder",
            uri: metadataUri,
            plugins: [
                {
                    type: "Royalties",
                    basisPoints: 500,
                    creators: [
                        { address: signer.publicKey, percentage: 100 },
                    ],
                    ruleSet: ruleSet("None"),
                },
            ],
        }).sendAndConfirm(umi);

        const signature = base58.deserialize(tx.signature)[0];

        console.log(`signature ${signature} , asset : ${asset.publicKey}`);
    } catch (e) {
        console.log(`error ${e}`);
    }
})();
