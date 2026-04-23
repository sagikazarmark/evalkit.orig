import { acquisitionPlugin, runPlugin } from "../src/index.ts";

const acquire = acquisitionPlugin("echo-acquisition", {
  capabilities: ["structured-errors"],
})((input) => `echo::${String(input)}`);

void runPlugin(acquire);
