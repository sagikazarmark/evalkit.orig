export const PLUGIN_PROTOCOL_VERSION = "2";

const pluginSpecSymbol = Symbol.for("evalkit.plugin.spec");

export type PluginKind = "source" | "scorer";
export type PluginCapability = string;

export interface PluginSpec {
  kind: PluginKind;
  name: string;
  version: string;
  capabilities: PluginCapability[];
}

export interface ScorerRequestMetadata {
  [key: string]: unknown;
}

type PluginWithSpec<F extends (...args: never[]) => unknown> = F & {
  [pluginSpecSymbol]: PluginSpec;
};

export type SourcePlugin = PluginWithSpec<
  (input: unknown) => unknown | Promise<unknown>
>;

export type ScorerPlugin = PluginWithSpec<
  (
    input: unknown,
    output: unknown,
    reference: unknown,
    runId: string | undefined,
    sampleId: string | undefined,
    trialIndex: number,
    metadata: ScorerRequestMetadata,
  ) => unknown | Promise<unknown>
>;

export class PluginError extends Error {
  readonly code: string;
  readonly details: unknown;

  constructor(code: string, message: string, details: unknown = {}) {
    super(message);
    this.name = "PluginError";
    this.code = code;
    this.details = details;
  }
}

export function sourcePlugin(
  name: string,
  options: {
    version?: string;
    capabilities?: PluginCapability[];
  } = {},
): <F extends (input: unknown) => unknown | Promise<unknown>>(plugin: F) => SourcePlugin {
  return decoratePlugin("source", name, options);
}

export function scorerPlugin(
  name: string,
  options: {
    version?: string;
    capabilities?: PluginCapability[];
  } = {},
): <
  F extends (
    input: unknown,
    output: unknown,
    reference: unknown,
    runId: string | undefined,
    sampleId: string | undefined,
    trialIndex: number,
    metadata: ScorerRequestMetadata,
  ) => unknown | Promise<unknown>,
>(
  plugin: F,
) => ScorerPlugin {
  return decoratePlugin("scorer", name, options);
}

export async function runPlugin(plugin: SourcePlugin | ScorerPlugin): Promise<void> {
  const spec = plugin[pluginSpecSymbol];

  if (spec === undefined) {
    throw new TypeError(
      "plugin must be decorated with sourcePlugin(...) or scorerPlugin(...)",
    );
  }

  const request = await readRequest();
  writeJson({
    kind: spec.kind,
    name: spec.name,
    version: spec.version,
    schema_version: PLUGIN_PROTOCOL_VERSION,
    capabilities: spec.capabilities,
  });

  try {
    if (spec.kind === "source") {
      const output = await (plugin as SourcePlugin)(request.input);
      writeJson({ output });
      return;
    }

    const score = await (plugin as ScorerPlugin)(
      request.input,
      request.output,
      request.reference,
      asOptionalString(request.run_id),
      asOptionalString(request.sample_id),
      asTrialIndex(request.trial_index),
      asMetadata(request.metadata),
    );
    writeJson({ score });
  } catch (error) {
    if (error instanceof PluginError) {
      writeJson({
        error: {
          code: error.code,
          message: error.message,
          details: error.details ?? {},
        },
      });
      return;
    }

    throw error;
  }
}

function decoratePlugin<K extends PluginKind>(
  kind: K,
  name: string,
  options: {
    version?: string;
    capabilities?: PluginCapability[];
  },
) {
  const spec: PluginSpec = {
    kind,
    name,
    version: options.version ?? "0.1.0",
    capabilities: options.capabilities ?? [],
  };

  return <F extends (...args: never[]) => unknown>(plugin: F): PluginWithSpec<F> => {
    const decorated = plugin as PluginWithSpec<F>;
    decorated[pluginSpecSymbol] = spec;
    return decorated;
  };
}

async function readRequest(): Promise<Record<string, unknown>> {
  const chunks: Buffer[] = [];

  for await (const chunk of process.stdin) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  }

  const input = Buffer.concat(chunks).toString("utf8").trim();
  if (input.length === 0) {
    throw new Error("expected one JSON request line on stdin");
  }

  const payload = JSON.parse(input) as unknown;
  if (payload === null || typeof payload !== "object" || Array.isArray(payload)) {
    throw new TypeError("plugin request must be a JSON object");
  }

  return payload as Record<string, unknown>;
}

function writeJson(payload: Record<string, unknown>): void {
  process.stdout.write(`${JSON.stringify(payload)}\n`);
}

function asOptionalString(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function asTrialIndex(value: unknown): number {
  return typeof value === "number" && Number.isInteger(value) ? value : 0;
}

function asMetadata(value: unknown): ScorerRequestMetadata {
  if (value === null || value === undefined || typeof value !== "object" || Array.isArray(value)) {
    return {};
  }

  return value as ScorerRequestMetadata;
}
