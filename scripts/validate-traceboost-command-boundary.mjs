#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

const repoRoot = process.cwd();
const policyPath = path.join(repoRoot, "apps/traceboost-demo/desktop-command-boundary.json");
const policy = JSON.parse(fs.readFileSync(policyPath, "utf8"));

const backendPath = path.join(repoRoot, policy.backend_handler_file);
const bridgePath = path.join(repoRoot, policy.frontend_bridge_file);
const generatedBridgeStubPath = path.join(repoRoot, policy.generated_bridge_stub_file);

const backendSource = fs.readFileSync(backendPath, "utf8");
const bridgeSource = fs.readFileSync(bridgePath, "utf8");
const generatedBridgeStubSource = fs.readFileSync(generatedBridgeStubPath, "utf8");

function parseBackendHandlerCommands(source) {
  const match = source.match(/generate_handler!\s*\[([\s\S]*?)\]\s*\)/m);
  if (!match) {
    throw new Error("could not locate tauri::generate_handler! command list");
  }
  return match[1]
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean)
    .sort();
}

function parseBridgeAwareCommands(source) {
  return [...new Set([...source.matchAll(/"([a-z0-9_]+_command)"/g)].map((match) => match[1]))].sort();
}

function parseGeneratedBridgeCommands(source) {
  const match = source.match(/export const desktopBridgeCommands = \{([\s\S]*?)\} as const;/m);
  if (!match) {
    throw new Error("could not locate desktopBridgeCommands in generated bridge stubs");
  }
  return [...new Set([...match[1].matchAll(/"([a-z0-9_]+_command)"/g)].map((entry) => entry[1]))].sort();
}

function classifyCommands(commands, groups) {
  const errors = [];
  const matches = new Map();
  for (const command of commands) {
    const matchedGroups = groups.filter((group) => new RegExp(group.pattern).test(command));
    if (matchedGroups.length === 0) {
      errors.push(`command \`${command}\` does not match any command group`);
      continue;
    }
    if (matchedGroups.length > 1) {
      errors.push(
        `command \`${command}\` matches multiple command groups: ${matchedGroups.map((group) => group.id).join(", ")}`
      );
      continue;
    }
    matches.set(command, matchedGroups[0].id);
  }
  return { errors, matches };
}

const backendCommands = parseBackendHandlerCommands(backendSource);
const generatedBridgeCommands = parseGeneratedBridgeCommands(generatedBridgeStubSource);
const bridgeCommands = [
  ...new Set([...parseBridgeAwareCommands(bridgeSource), ...generatedBridgeCommands])
].sort();
const backendOnlyCommands = policy.backend_only_commands.map((entry) => entry.command).sort();

const errors = [];

for (const command of bridgeCommands) {
  if (!backendCommands.includes(command)) {
    errors.push(`bridge-aware command \`${command}\` is not registered in tauri::generate_handler!`);
  }
}

for (const command of backendCommands) {
  if (!bridgeCommands.includes(command) && !backendOnlyCommands.includes(command)) {
    errors.push(
      `backend handler command \`${command}\` is neither bridge-aware nor listed as backend-only in desktop-command-boundary.json`
    );
  }
}

for (const command of backendOnlyCommands) {
  if (!backendCommands.includes(command)) {
    errors.push(`backend-only command \`${command}\` is not present in the tauri handler list`);
  }
  if (bridgeCommands.includes(command)) {
    errors.push(`backend-only command \`${command}\` is still referenced in the frontend bridge`);
  }
}

const backendClassification = classifyCommands(backendCommands, policy.command_groups);
errors.push(...backendClassification.errors);

const matchedGroups = new Set(backendClassification.matches.values());
for (const group of policy.command_groups) {
  if (!matchedGroups.has(group.id)) {
    errors.push(`command group \`${group.id}\` does not match any backend handler command`);
  }
}

if (errors.length > 0) {
  console.error("TraceBoost desktop command boundary violations:");
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}

const groupCounts = new Map();
for (const groupId of backendClassification.matches.values()) {
  groupCounts.set(groupId, (groupCounts.get(groupId) ?? 0) + 1);
}

console.log("TraceBoost desktop command boundary is consistent.");
console.log(`- backend handler commands: ${backendCommands.length}`);
console.log(`- bridge-aware commands: ${bridgeCommands.length}`);
console.log(`- backend-only commands: ${backendOnlyCommands.length}`);
for (const group of policy.command_groups) {
  const count = groupCounts.get(group.id) ?? 0;
  console.log(`- ${group.id}: ${count}`);
}
