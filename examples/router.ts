import "dotenv/config";
import { Contract } from ".";

Contract.init().then(async () => {
  const ret = await Contract.router.simulateSwapOperations({
    offerAmount: "10000000",
    operations: [
      {
        orai_swap: {
          offer_asset_info: {
            token: {
              contract_addr: process.env.ORAIX_CONTRACT,
            },
          },
          ask_asset_info: {
            native_token: { denom: process.env.DENOM },
          },
        },
      },
    ],
  });

  console.log(ret);
});
