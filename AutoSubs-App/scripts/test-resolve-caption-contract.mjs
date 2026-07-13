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
const tauriConfig = JSON.parse(
  readFileSync(
    new URL("src-tauri/tauri.conf.json", new URL("../", import.meta.url)),
    "utf8",
  ),
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
  /local comp, compErr = ensure_fusion_composition\(timelineItem, isAnimated\)[\s\S]*?if not comp then[\s\S]*?error\(compErr\)/,
  "Resolve Lua bridge must fail subtitle placement only when composition recovery fails",
);
assert.ok(
  lua.indexOf("local ensure_fusion_composition") < lua.indexOf("local function apply_subtitle_text"),
  "the Fusion recovery function must be declared before apply_subtitle_text captures it",
);
assert.match(
  lua,
  /ensure_fusion_composition = function[\s\S]*?AddFusionComp[\s\S]*?Paste[\s\S]*?MediaOut/,
  "Resolve Lua bridge must rebuild a missing animated-caption Fusion composition",
);
assert.match(
  lua,
  /ensure_fusion_composition\(timelineItem, isAnimated\)/,
  "subtitle placement must invoke the Fusion composition recovery path",
);
assert.equal(
  tauriConfig.bundle.resources["../../Resolve-Integration/autosubs-macro.setting"],
  "resources/AutoSubs/autosubs-macro.setting",
  "the canonical caption macro must be bundled for runtime composition recovery",
);
assert.match(
  lua,
  /local function find_render_job_by_id[\s\S]*?JobId/,
  "audio export must find the render job by the PID returned from AddRenderJob",
);
assert.match(
  lua,
  /if not pid or pid == "" then[\s\S]*?AddRenderJob returned no job ID/,
  "audio export must reject an empty render job ID",
);
assert.match(
  lua,
  /local renderStarted = project:StartRendering\(pid\)[\s\S]*?if not renderStarted then/,
  "audio export must reject StartRendering failures",
);

console.log("Resolve caption contract verified");
