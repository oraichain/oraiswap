import "dotenv/config";
import { Contract } from ".";

Contract.init().then(async () => {
  const ret = await Contract.oracle.taxRate();

  console.log(ret);
});
