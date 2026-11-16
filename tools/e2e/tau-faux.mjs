// A tau server whose only model is pi-ai's faux provider `faux/echo`.
// Same startup as `tau` (core/src/command/command.ts serve()), but with a scripted model
// instead of builtinModels, so a conversation actually runs without credentials.
//
// Run with the node and node_modules of the nix `tau` package; see run-server.sh (TAU_FAUX=1).
// usage: node tau-faux.mjs [--port PORT]      home is $HOME/.tau, as for `tau`.
//
// Each model request answers "echo: <last user text>", unless TAU_FAUX_SCRIPT names a file
// holding a JSON array of steps. Each request then takes the first step and writes the rest
// back, so a test can (re)write the file at any time, e.g. just before an `input`:
//   "some text"                              -> a text answer
//   {"tool": "record_forecast", "arguments": {...}}  -> a tool call (stopReason toolUse)
// An empty or missing file falls back to echo.
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import {
	createModels,
	fauxAssistantMessage,
	fauxProvider,
	fauxToolCall,
} from "@earendil-works/pi-ai";
import { NodeExecutionEnv } from "@earendil-works/pi-durable/env/node";
import { fromDirectory, startServer } from "tau/server";

const portAt = process.argv.indexOf("--port");
const port = portAt === -1 ? 6283 : Number(process.argv[portAt + 1]);
const home = path.join(homedir(), ".tau");
const scriptFile = process.env.TAU_FAUX_SCRIPT;
const nextStep = () => {
	if (!scriptFile || !existsSync(scriptFile)) return undefined;
	const steps = JSON.parse(readFileSync(scriptFile, "utf8") || "[]");
	if (steps.length === 0) return undefined;
	writeFileSync(scriptFile, JSON.stringify(steps.slice(1)));
	return steps[0];
};

const lastUserText = (context) => {
	const user = [...context.messages].reverse().find((m) => m.role === "user");
	if (user === undefined) return "";
	return typeof user.content === "string"
		? user.content
		: user.content.map((c) => (c.type === "text" ? c.text : "")).join("");
};
const answer = (context) => {
	const step = nextStep();
	if (step === undefined) return fauxAssistantMessage(`echo: ${lastUserText(context)}`);
	if (typeof step === "string") return fauxAssistantMessage(step);
	return fauxAssistantMessage([fauxToolCall(step.tool, step.arguments ?? {})], {
		stopReason: "toolUse",
	});
};

// tokensPerSecond makes faux stream text in pieces, so `open` shows `a` ops on
// docs/pi.live/generation like a real provider does.
const faux = fauxProvider({
	provider: "faux",
	models: [{ id: "echo", name: "Faux echo" }],
	tokensPerSecond: 50,
});
// The faux provider consumes one step per request; keep the queue topped up.
const refill = () => faux.appendResponses(Array.from({ length: 1000 }, () => answer));
refill();
setInterval(() => {
	if (faux.getPendingResponseCount() < 100) refill();
}, 1000).unref();

const models = createModels();
models.setProvider(faux.provider);
const server = await startServer({
	home,
	source: fromDirectory(path.join(home, "extensions")),
	models,
	port,
	env: (target) => new NodeExecutionEnv({ cwd: target.cwd ?? process.cwd() }),
});
for (const { project, reason } of server.skipped)
	process.stderr.write(`tau: skipped ${project}: ${reason}\n`);
process.stdout.write(`tau: listening on http://127.0.0.1:${server.address.port}\n`);
const stop = async () => {
	process.once("SIGINT", () => process.exit(1));
	process.once("SIGTERM", () => process.exit(1));
	await server.close();
	process.exit(0);
};
process.once("SIGINT", stop);
process.once("SIGTERM", stop);
