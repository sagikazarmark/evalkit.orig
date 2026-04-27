import { sourcePlugin, runPlugin } from "../src/index.ts";

const acquire = sourcePlugin("echo-source", {
  capabilities: ["structured-errors"],
})((input) => `echo::${String(input)}`);

void runPlugin(acquire);
