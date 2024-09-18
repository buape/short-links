// Import the Carbon client
import { Client, ClientMode } from "@buape/carbon";
import type { ExecutionContext } from "@cloudflare/workers-types";
import LinksCommand from "./commands/ping";
import PinCommand from "./commands/links";

type Env = {
  CLIENT_ID: string;
  PUBLIC_KEY: string;
  DISCORD_TOKEN: string;
};

export default {
  async fetch(request: Request, env: Env, ctx: ExecutionContext) {
    console.log(env.CLIENT_ID);
    // Create a new Carbon client.
    const client = new Client(
      {
        // Put your client ID, public key, and token here, _env will have them from your `wrangler.toml` file.
        clientId: env.CLIENT_ID,
        publicKey: env.PUBLIC_KEY,
        token: env.DISCORD_TOKEN,
        mode: ClientMode.CloudflareWorkers,
      },
      [new LinksCommand(), new PinCommand()]
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