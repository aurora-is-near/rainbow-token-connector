require("dotenv").config();
require("@nomicfoundation/hardhat-ethers");
require("@openzeppelin/hardhat-upgrades");

const AURORA_PRIVATE_KEY = process.env.AURORA_PRIVATE_KEY;
const ETHERSCAN_API_KEY = process.env.ETHERSCAN_API_KEY;

task("deploy", "Deploy silo to silo proxy contract")
  .addParam("silo", "Config file name without extension")
  .setAction(async (taskArgs, hre) => {
    const { deploy } = require("./utils/scripts.js");
    const [signer] = await hre.ethers.getSigners();
    const config = require(`./configs/${taskArgs.silo}.json`);

    await hre.run("compile");
    await deploy({
      signer,
      wNearAddress: config.wNearAddress,
      siloAccountId: config.siloAccountId,
      nativeTokenAccountId: config.nativeTokenAccountId,
      auroraSdkAddress: config.auroraSdkAddress,
      auroraUtilsAddress: config.auroraUtilsAddress,
    });
  });

task("upgrade", "Upgrade silo to silo proxy contract")
  .addParam("silo", "Config file name without extension")
  .setAction(async (taskArgs, hre) => {
    const { upgrade } = require("./utils/scripts.js");
    const [signer] = await hre.ethers.getSigners();
    const config = require(`./configs/${taskArgs.silo}.json`);

    await hre.run("compile");
    await upgrade({
      signer,
      proxyAddress: config.siloToSiloProxyAddress,
      auroraSdkAddress: config.auroraSdkAddress,
      auroraUtilsAddress: config.auroraUtilsAddress,
    });
  });

task(
  "register_token",
  'Registers a binding of "nearTokenAccountId:auroraTokenAddress" in "SiloToSilo" contract.',
)
  .addParam("silo", "Config file name without extension")
  .addParam("auroraTokenAddress", "Token address on Aurora")
  .setAction(async (taskArgs) => {
    const { registerToken } = require("./utils/scripts.js");
    const [signer] = await hre.ethers.getSigners();
    const config = require(`./configs/${taskArgs.silo}.json`);

    await registerToken(signer, config, taskArgs.auroraTokenAddress);
  });

task("get_token_near_account_id", "Get Token Near Account Id from SiloToSilo")
  .addParam("silo", "Config file name without extension")
  .addParam("auroraTokenAddress", "Token address on Aurora")
  .setAction(async (taskArgs) => {
    const { getTokenNearAccountId } = require("./utils/scripts.js");
    const [signer] = await hre.ethers.getSigners();
    const config = require(`./configs/${taskArgs.silo}.json`);

    await getTokenNearAccountId(signer, config, taskArgs.auroraTokenAddress);
  });

task(
  "storage_deposit",
  'Puts a storage deposit in "nearTokenAccountId" for the "SiloToSilo" implicit NEAR Account ID.',
)
  .addParam("silo", "Config file name without extension")
  .addParam("auroraTokenAddress", "Token address on Aurora")
  .setAction(async (taskArgs) => {
    const { storageDeposit } = require("./utils/scripts.js");
    const [signer] = await hre.ethers.getSigners();
    const config = require(`./configs/${taskArgs.silo}.json`);

    await storageDeposit(signer, config, taskArgs.auroraTokenAddress);
  });

task("is_storage_registered", "Check is storage registered for token")
  .addParam("silo", "Config file name without extension")
  .addParam("auroraTokenAddress", "Token address on Aurora")
  .setAction(async (taskArgs) => {
    const { isStorageRegistered } = require("./utils/scripts.js");
    const [signer] = await hre.ethers.getSigners();
    const config = require(`./configs/${taskArgs.silo}.json`);

    await isStorageRegistered(signer, config, taskArgs.auroraTokenAddress);
  });

task("safe_ft_transfer_call_to_near", "Init ft transfer call on Near")
  .addParam("silo", "Config file name without extension")
  .addParam("receiverId", "Receiver Id")
  .addParam("amount", "Transferred tokens amount")
  .addParam("msg", "Msg for ft_transfer_call")
  .addParam("auroraTokenAddress", "Token address on Aurora")
  .setAction(async (taskArgs) => {
    const { safeFtTransferCallToNear } = require("./utils/scripts.js");
    const [signer] = await hre.ethers.getSigners();
    const config = require(`./configs/${taskArgs.silo}.json`);

    await safeFtTransferCallToNear(
      signer,
      config,
      taskArgs.auroraTokenAddress,
      taskArgs.receiverId,
      taskArgs.amount,
      taskArgs.msg,
    );
  });

task("ft_transfer_call_to_near", "Init ft transfer call on Near")
  .addParam("silo", "Config file name without extension")
  .addParam("receiverId", "Receiver Id")
  .addParam("amount", "Transferred tokens amount")
  .addParam("msg", "Msg for ft_transfer_call")
  .addParam("auroraTokenAddress", "Token address on Aurora")
  .setAction(async (taskArgs) => {
    const { ftTransferCallToNear } = require("./utils/scripts.js");
    const [signer] = await hre.ethers.getSigners();
    const config = require(`./configs/${taskArgs.silo}.json`);

    await ftTransferCallToNear(
      signer,
      config,
      taskArgs.auroraTokenAddress,
      taskArgs.receiverId,
      taskArgs.amount,
      taskArgs.msg,
    );
  });

task("recipient_storage_deposit", "Storage Deposit for Recipient")
  .addParam("silo", "Config file name without extension")
  .addParam("receiverId", "Receiver Id")
  .addParam("auroraTokenAddress", "Token address on Aurora")
  .setAction(async (taskArgs) => {
    const { recipientStorageDeposit } = require("./utils/scripts.js");
    const [signer] = await hre.ethers.getSigners();
    const config = require(`./configs/${taskArgs.silo}.json`);

    await recipientStorageDeposit(
      signer,
      config,
      taskArgs.auroraTokenAddress,
      taskArgs.receiverId,
    );
  });

task(
  "is_recipient_storage_registered",
  "Check if Storage Deposited for Recipient",
)
  .addParam("silo", "Config file name without extension")
  .addParam("receiverId", "Receiver Id")
  .addParam("auroraTokenAddress", "Token address on Aurora")
  .setAction(async (taskArgs) => {
    const { isRecipientStorageRegistered } = require("./utils/scripts.js");
    const [signer] = await hre.ethers.getSigners();
    const config = require(`./configs/${taskArgs.silo}.json`);

    await isRecipientStorageRegistered(
      signer,
      config,
      taskArgs.auroraTokenAddress,
      taskArgs.receiverId,
    );
  });

task("get_user_balance", "Get user balance")
  .addParam("silo", "Config file name without extension")
  .addParam("auroraTokenAddress", "Token address on Aurora")
  .setAction(async (taskArgs) => {
    const { getUserBalance } = require("./utils/scripts.js");
    const [signer] = await hre.ethers.getSigners();
    const config = require(`./configs/${taskArgs.silo}.json`);

    await getUserBalance(signer, config, taskArgs.auroraTokenAddress);
  });

task("withdraw", "Withdraw tokens")
  .addParam("silo", "Config file name without extension")
  .addParam("auroraTokenAddress", "Token address on Aurora")
  .setAction(async (taskArgs) => {
    const { withdraw } = require("./utils/scripts.js");
    const [signer] = await hre.ethers.getSigners();
    const config = require(`./configs/${taskArgs.silo}.json`);

    await withdraw(signer, config, taskArgs.auroraTokenAddress);
  });

task("deploy-sdk", "Deploy aurora sdk").setAction(async (_, hre) => {
  const { deploySDK } = require("./utils/scripts.js");
  const [signer] = await hre.ethers.getSigners();

  await hre.run("compile");
  await deploySDK({
    signer,
  });
});

module.exports = {
  solidity: {
    version: "0.8.21",
    settings: {
      optimizer: {
        enabled: true,
        runs: 200,
      },
    },
  },
  networks: {
    mainnet_aurora: {
      url: "https://mainnet.aurora.dev",
      accounts: [`0x${AURORA_PRIVATE_KEY}`],
      chainId: 1313161554,
    },
    testnet_aurora: {
      url: "https://testnet.aurora.dev",
      accounts: [`0x${AURORA_PRIVATE_KEY}`],
      chainId: 1313161555,
    },
    develop_aurora: {
      url: "https://develop.rpc.testnet.aurora.dev:8545",
      accounts: [`0x${AURORA_PRIVATE_KEY}`],
    },
    mainnet_enpower: {
      url: "http://powergold.aurora.dev",
      accounts: [`0x${AURORA_PRIVATE_KEY}`],
      chainId: 1313161560,
    },
  },
  etherscan: {
    apiKey: {
      mainnet_aurora: `${ETHERSCAN_API_KEY}`,
      testnet_aurora: `${ETHERSCAN_API_KEY}`,
      mainnet_enpower: `${ETHERSCAN_API_KEY}`,
    },
    customChains: [
      {
        network: "mainnet_aurora",
        chainId: 1313161554,
        urls: {
          apiURL: "https://explorer.mainnet.aurora.dev/api",
          browserURL: "https://explorer.mainnet.aurora.dev",
        },
      },
      {
        network: "testnet_aurora",
        chainId: 1313161555,
        urls: {
          apiURL: "https://explorer.testnet.aurora.dev/api",
          browserURL: "https://explorer.testnet.aurora.dev",
        },
      },
      {
        network: "mainnet_enpower",
        chainId: 1313161560,
        urls: {
          apiURL: "https://explorer.powergold.aurora.dev/api",
          browserURL: "https://explorer.powergold.aurora.dev",
        },
      },
    ],
  },
  mocha: {
    timeout: 100000000,
  },
};
