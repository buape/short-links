// Import the Carbon client
import { Client, ClientMode } from "@buape/carbon";
import type { ExecutionContext, KVNamespace } from "@cloudflare/workers-types";
import PingCommand from "./commands/ping";
import LinksRootCommand from "./commands/links";

export type Env = {
  CLIENT_ID: string;
  PUBLIC_KEY: string;
  DISCORD_TOKEN: string;
  SHORT_LINKS: KVNamespace;
  ACCESS_KEY: string;
  INTERNAL_LOGS_WEBHOOK: string;
};

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext) {
    // Create a new Carbon client.
    const client = new Client(
      {
        // Put your client ID, public key, and token here, _env will have them from your `wrangler.toml` file.
        clientId: env.CLIENT_ID,
        publicKey: env.PUBLIC_KEY,
        token: env.DISCORD_TOKEN,
        mode: ClientMode.CloudflareWorkers,
      },
      [new PingCommand(), new LinksRootCommand(env)]
    );
    client.router.get("/deploy", async () => {
      await client.deployCommands();
      return new Response("Deployed commands");
    });
    // This is how you pass control from your cf worker to Carbon. Make sure you pass the ctx part as well, so that awaits work properly within Carbon.
    const response = await client.router.fetch(request, ctx);
    // Finally, return the response from Carbon.
    return response;
  },
};

export const sendToInternalLogs = async (message: string, env: Env) => {
  await fetch(env.INTERNAL_LOGS_WEBHOOK, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      content: message,
      allowed_mentions: { parse: [] },
    }),
  }).catch((e) => {
    console.error(`Error sending to internal logs: ${e.message}`);
  });
};
