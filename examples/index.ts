import { CosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { contracts } from "../build";
import { OraiswapConverterClient } from "../build/OraiswapConverter.client";
import { OraiswapFactoryClient } from "../build/OraiswapFactory.client";
import { OraiswapOracleClient } from "../build/OraiswapOracle.client";
import { OraiswapPairClient } from "../build/OraiswapPair.client";
import { OraiswapRewarderClient } from "../build/OraiswapRewarder.client";
import { OraiswapRouterClient } from "../build/OraiswapRouter.client";
import { OraiswapStakingClient } from "../build/OraiswapStaking.client";
import { OraiswapTokenClient } from "../build/OraiswapToken.client";

type ContractName =
  | "oracle"
  | "factory"
  | "router"
  | "staking"
  | "rewarder"
  | "converter"
  | "pair"
  | "token";

export class Contract {
  private static _sender: string = null;
  private static _client: any = null;

  static set client(client: any) {
    this._client = client;
  }

  static set sender(sender: string) {
    this._sender = sender;
  }

  private static getContract(type: ContractName, address: string): any {
    const key = "_" + type;
    const className = type.charAt(0).toUpperCase() + type.slice(1);

    if (!this[key]) {
      this[key] = new contracts[`Oraiswap${className}`][
        `Oraiswap${className}Client`
      ](this._client, this._sender, address);
    } else {
      this[key].sender = this._sender;
      this[key].contractAddress = address;
    }
    return this[key];
  }

  static get oracle(): OraiswapOracleClient {
    return this.getContract("oracle", process.env.ORACLE_CONTRACT);
  }

  static get factory(): OraiswapFactoryClient {
    return this.getContract("factory", process.env.FACTORY_CONTRACT);
  }

  static get router(): OraiswapRouterClient {
    return this.getContract("router", process.env.ROUTER_CONTRACT);
  }

  static get staking(): OraiswapStakingClient {
    return this.getContract("staking", process.env.STAKING_CONTRACT);
  }

  static get rewarder(): OraiswapRewarderClient {
    return this.getContract("rewarder", process.env.REWARDER_CONTRACT);
  }

  static get converter(): OraiswapConverterClient {
    return this.getContract("converter", process.env.CONVERTER_CONTRACT);
  }

  static pair(contractAddress: string): OraiswapPairClient {
    return this.getContract("pair", contractAddress);
  }

  static token(contractAddress: string): OraiswapTokenClient {
    return this.getContract("token", contractAddress);
  }

  static async init() {
    Contract.client = await CosmWasmClient.connect(process.env.RPC_URL);
  }
}
