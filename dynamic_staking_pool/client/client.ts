console.log("My address:", pg.wallet.publicKey.toString());

// Fetch the wallet's balance
const balance = await pg.connection.getBalance(pg.wallet.publicKey);
console.log(`My balance: ${balance / web3.LAMPORTS_PER_SOL} SOL`);

try {
  // Fetch and log the recent blockhash
  const { blockhash } = await pg.connection.getLatestBlockhash();
  console.log("Recent Blockhash:", blockhash);

  // Fetch and log transaction count (without publicKey as an argument)
  const transactionCount = await pg.connection.getTransactionCount();
  console.log(`Transaction count: ${transactionCount}`);
} catch (error) {
  console.error("Failed to get recent blockhash or transaction count:", error);
}

// Example function to send lamports
async function sendLamports(toPubkey: web3.PublicKey, amount: number) {
  // Create a transfer transaction
  const tx = new web3.Transaction().add(
    web3.SystemProgram.transfer({
      fromPubkey: pg.wallet.publicKey,
      toPubkey,
      lamports: amount * web3.LAMPORTS_PER_SOL, // Convert SOL to lamports
    })
  );

  // Get the latest blockhash to include in the transaction
  const { blockhash } = await pg.connection.getLatestBlockhash();
  tx.recentBlockhash = blockhash;

  // Sign the transaction with the wallet
  const signedTx = await pg.wallet.signTransaction(tx);

  // Send the signed transaction
  const signature = await pg.connection.sendRawTransaction(signedTx.serialize());
  console.log(`Transaction signature: ${signature}`);

  // Confirm the transaction
  await pg.connection.confirmTransaction(signature);
  console.log("Transaction confirmed.");
}

// Replace with a recipient public key to test sending lamports
const recipientPubkey = new web3.PublicKey("RecipientPublicKeyHere"); // Set recipient's public key here
await sendLamports(recipientPubkey, 0.1); // Sends 0.1 SOL
