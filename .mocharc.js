require("./.pnp.cjs").setup();

module.exports = {
  timeout: 30_000,
  require: [require.resolve("ts-node/register")],
};
