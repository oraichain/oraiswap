import "dotenv/config";
import { Contract } from ".";

Contract.init().then(async () => {
  const tokenClient = Contract.token(process.env.ORAIX_CONTRACT);

  const accounts = await tokenClient.allAccounts({ limit: 10 });

  console.log(accounts);
});
