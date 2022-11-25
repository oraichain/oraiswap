import "dotenv/config";
import { Contract } from ".";

Contract.init().then(async () => {
  const ret = await Contract.oracle.treasury("tax_rate", {});

  console.log(ret);
});
