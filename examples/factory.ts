import "dotenv/config";
import { Contract } from ".";

Contract.init().then(async () => {
  const { pairs } = await Contract.factory.pairs({ limit: 10 });

  const ret = await Promise.all(
    pairs.map((pair) => Contract.pair(pair.contract_addr).pair())
  );
  console.log(JSON.stringify(ret, null, 2));
});
