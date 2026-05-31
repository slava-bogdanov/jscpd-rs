#!/usr/bin/env node
"use strict";

const { runBinary } = require("../lib/run-binary");

runBinary("jscpd", process.argv.slice(2));
