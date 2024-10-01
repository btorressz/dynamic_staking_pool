import { TOKEN_PROGRAM_ID } from "@solana/spl-token"; // Only TOKEN_PROGRAM_ID is needed
import { BN } from "@project-serum/anchor";

describe("Dynamic Staking Pool", () => {
  const poolAccountKp = new web3.Keypair(); // Keypair for the pool account
  const userStakerKp = new web3.Keypair(); // Keypair for the user staking

  it("initialize", async () => {
    // Set the reward rate
    const rewardRate = new BN(10);

    // Send transaction to initialize the staking pool
    const txHash = await pg.program.methods
      .initialize(rewardRate)
      .accounts({
        poolAccount: poolAccountKp.publicKey,
        initializer: pg.wallet.publicKey, // Use the test wallet as the initializer
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([poolAccountKp])
      .rpc();
    console.log(`Transaction hash: ${txHash}`);

    // Confirm the transaction
    await pg.connection.confirmTransaction(txHash);

    // Fetch and verify the PoolAccount state
    const poolAccount = await pg.program.account.poolAccount.fetch(
      poolAccountKp.publicKey
    );
    console.log("On-chain Pool Account Data:", poolAccount);

    // Verify the reward rate
    assert.ok(poolAccount.rewardRate.eq(rewardRate));
    assert.ok(poolAccount.totalStaked.eq(new BN(0))); // Should be zero initially
  });

  it("stake", async () => {
    const stakingAmount = new BN(50); // Set the staking amount

    // Create a user token account for staking
    const userTokenAccount = await createTokenAccount(pg, userStakerKp.publicKey);

    // Pool's token account
    const poolTokenAccount = await createTokenAccount(pg, poolAccountKp.publicKey);

    // Send transaction to stake tokens
    const txHash = await pg.program.methods
      .stake(stakingAmount)
      .accounts({
        userStake: userStakerKp.publicKey, // Derived PDA for the user's stake
        poolAccount: poolAccountKp.publicKey,
        staker: userStakerKp.publicKey, // The user staking
        userTokenAccount: userTokenAccount, // User's token account for staking
        poolTokenAccount: poolTokenAccount, // Pool's token account
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
      })
      .signers([userStakerKp])
      .rpc();
    console.log(`Transaction hash: ${txHash}`);

    // Confirm the transaction
    await pg.connection.confirmTransaction(txHash);

    // Fetch and verify the UserStake state
    const userStake = await pg.program.account.userStake.fetch(userStakerKp.publicKey);
    console.log("On-chain User Stake Data:", userStake);

    // Verify staked amount
    assert.ok(userStake.amountStaked.eq(stakingAmount));
  });

  it("claim_rewards", async () => {
    // Send transaction to claim rewards
    const txHash = await pg.program.methods
      .claimRewards()
      .accounts({
        userStake: userStakerKp.publicKey, // User's staking account
        poolAccount: poolAccountKp.publicKey,
        userTokenAccount: await createTokenAccount(pg, userStakerKp.publicKey), // The user's token account for rewards
        rewardMint: await createMint(pg), // Mint for the reward tokens
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([userStakerKp])
      .rpc();
    console.log(`Transaction hash: ${txHash}`);

    // Confirm the transaction
    await pg.connection.confirmTransaction(txHash);

    // Fetch and verify the updated UserStake state
    const userStake = await pg.program.account.userStake.fetch(userStakerKp.publicKey);
    console.log("Updated On-chain User Stake Data:", userStake);

    // Check the last claim time has been updated (non-zero)
    assert.ok(userStake.lastClaimTime.gt(new BN(0)));
  });
});

// Utility to create a token account for staking
async function createTokenAccount(pg, ownerPubkey) {
  const tokenAccount = new web3.Keypair();
  await pg.program.provider.send(
    new web3.Transaction().add(
      web3.SystemProgram.createAccount({
        fromPubkey: pg.wallet.publicKey,
        newAccountPubkey: tokenAccount.publicKey,
        lamports: await pg.provider.connection.getMinimumBalanceForRentExemption(165),
        space: 165,
        programId: TOKEN_PROGRAM_ID,
      })
    ),
    [tokenAccount]
  );
  return tokenAccount.publicKey;
}

// Utility to create a mint for rewards
async function createMint(pg) {
  const mintAccount = new web3.Keypair();
  await pg.program.provider.send(
    new web3.Transaction().add(
      web3.SystemProgram.createAccount({
        fromPubkey: pg.wallet.publicKey,
        newAccountPubkey: mintAccount.publicKey,
        lamports: await pg.provider.connection.getMinimumBalanceForRentExemption(82),
        space: 82,
        programId: TOKEN_PROGRAM_ID,
      })
    ),
    [mintAccount]
  );
  return mintAccount.publicKey;
}
