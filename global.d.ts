declare global {
  namespace NodeJS {
    interface ProcessEnv {
      CHAIN_ID: "Oraichain" | "Oraichain-fork" | "Oraichain-testnet";
      MNEMONIC: string;
      LCD_URL:
        | "https://lcd.orai.io"
        | "https://pre.lcd.orai.io"
        | "https://testnet.lcd.orai.io";
      DENOM: "orai";
      RPC_URL:
        | "https://rpc.orai.io"
        | "https://pre.rpc.orai.io"
        | "https://testnet.rpc.orai.io";
      PREFIX: "orai";

      // config for oraichain token
      AIRI_CONTRACT: string;
      ORAIX_CONTRACT: string;
      USDT_CONTRACT: string;
      KWT_CONTRACT: string;
      MILKY_CONTRACT: string;

      // config for oraichain contract
      FACTORY_CONTRACT: string;
      ROUTER_CONTRACT: string;
      ORACLE_CONTRACT: string;
      STAKING_CONTRACT: string;
      REWARDER_CONTRACT: string;
      CONVERTER_CONTRACT: string;
    }
  }
}

export {};
