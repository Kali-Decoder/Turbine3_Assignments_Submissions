import "dotenv/config";
import { createSignerFromKeypair, signerIdentity } from "@metaplex-foundation/umi";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import { irysUploader } from "@metaplex-foundation/umi-uploader-irys";

import wallet from "../../devnet-wallet.json";

const umi = createUmi(process.env.SOLANA_RPC_URL ?? "https://api.devnet.solana.com");

const keypair = umi.eddsa.createKeypairFromSecretKey(new Uint8Array(wallet));
const signer = createSignerFromKeypair(umi, keypair);

umi.use(
    irysUploader({
        address: "https://devnet.irys.xyz/",
    })
);

umi.use(signerIdentity(signer));

(async () => {
    try {
        // paste the image URI printed by nft_image.ts
        const image = "https://gateway.irys.xyz/HDbnDD5vAJgkik6fdxqZhtm44ojKohjJGtb5DZWdGRFA";

        const metadata = {
            name: "Kali-Decoder",
            description: "This side Kali-Decoder, the Turbin3 Assignment 2 submission.",
            image,
            attributes: [{ trait_type: "Unique", value: "Decoder" }],
            properties: {
                files: [
                    {
                        type: "image/jpeg",
                        uri: image,
                    },
                ],
                category: "image",
            },
        };

        const myUri = await umi.uploader.uploadJson(metadata);
        console.log(`metadata uri: ${myUri}`);
    } catch (error) {
        console.log("error", error);
    }
})();
