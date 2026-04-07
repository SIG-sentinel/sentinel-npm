#!/usr/bin/env node

const { runSentinel } = require("./_runner");

runSentinel(process.argv.slice(2));
