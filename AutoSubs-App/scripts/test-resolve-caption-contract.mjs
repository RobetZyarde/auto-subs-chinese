import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const lua = readFileSync(
  new URL("src-tauri/resources/modules/autosubs_core.lua", new URL("../", import.meta.url)),
  "utf8",
);
const macro = readFileSync(
  new URL("../Resolve-Integration/autosubs-macro.setting", new URL("../", import.meta.url)),
  "utf8",
);

assert.match(
  lua,
  /template:SetInput\("Text", subtitleText\)[\s\S]*?pcall\(clsTool\.SetInput, clsTool, "Text", subtitleText\)/,
  "Resolve Lua bridge must synchronize CharacterLevelStyling1.Text after setting the subtitle text",
);
assert.match(
  macro,
  /tool:SetInput\("StyledText", text\)[\s\S]*?cls:SetInput\("Text", text\)/,
  "Resolve macro must synchronize CharacterLevelStyling1.Text when its text changes",
);
assert.match(
  lua,
  /if not fusionCompCount or fusionCompCount <= 0 then[\s\S]*?error\("template clip has no Fusion composition/,
  "Resolve Lua bridge must fail subtitle placement when a clip has zero Fusion compositions",
);

console.log("Resolve caption contract verified");
