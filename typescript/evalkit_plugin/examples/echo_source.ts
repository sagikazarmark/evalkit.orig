import { sourcePlugin, runPlugin } from "../src/index.ts";

const produce = sourcePlugin("echo-source", {
  capabilities: ["structured-errors"],
})((input) => `echo::${String(input)}`);

void runPlugin(produce);
