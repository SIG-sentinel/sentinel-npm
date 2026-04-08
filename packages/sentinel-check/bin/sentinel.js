#!/usr/bin/env node

const { runSentinel } = require("./_runner");

runSentinel(process.argv.slice(2)).catch((error) => {
  console.error(`sentinel: ${error.message}`);
  process.exit(1);
});
