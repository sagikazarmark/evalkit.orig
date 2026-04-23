import { runPlugin, scorerPlugin } from "../src/index.ts";

const score = scorerPlugin("exact-match-scorer", {
  capabilities: ["structured-errors"],
})((input, output, reference) => {
  void input;

  return {
    type: "binary",
    value: reference !== undefined && output === reference,
  };
});

void runPlugin(score);
