import {
  type APIApplicationCommandBasicOption,
  ApplicationCommandOptionType,
  Command,
  type CommandInteraction,
  type CommandOptions,
  CommandWithSubcommands,
} from "@buape/carbon";
import type { Env } from "../index";

class CreateLink extends Command {
  private env: Env;

  constructor(env: Env) {
    super();
    this.env = env;
  }

  name = "create";
  description = "Create a short-link via Buape Link";
  options: APIApplicationCommandBasicOption[] = [
    {
      name: "domain",
      type: ApplicationCommandOptionType.String,
      description:
        "The domain this short-link should be created on (e.g. 'go.buape.com', 'go.kiai.app')",
      required: true,
      choices: [
        {
          name: "go.buape.com",
          value: "go-new.buape.com",
        },
        {
          name: "go.kiai.app",
          value: "go.kiai.app",
        },
      ],
    },
    {
      name: "url",
      type: ApplicationCommandOptionType.String,
      description: "The URL the short-link should redirect to",
      required: true,
    },
    {
      name: "slug",
      type: ApplicationCommandOptionType.String,
      description: "The slug for the short-link",
      required: true,
    },
  ];

  async run(interaction: CommandInteraction) {
    const domain = interaction.options.getString("domain");
    const url = interaction.options.getString("url");
    const slug = interaction.options.getString("slug");

    if (!domain || !url || !slug) {
      return interaction.reply({ content: "Missing required options" });
    }

    try {
      const response = await fetch(`https://${domain}/create`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${this.env.ACCESS_KEY}`,
        },
        body: JSON.stringify({ url, slug }),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const data = await response.json();
      return interaction.reply({
        content: `Short-link created: ${data.short_url}`,
      });
    } catch (error: any) {
      console.error("Error:", error);
      return interaction.reply({
        content: `Error creating short-link: ${error.message}`,
      });
    }
  }
}

class DeleteLink extends Command {
  private env: Env;

  constructor(env: Env) {
    super();
    this.env = env;
  }

  name = "sub2";
  description = "Subcommand 2";
  defer = true;

  async run(interaction: CommandInteraction) {
    interaction.reply({ content: "Subcommand 2" });
  }
}

export default class LinksRootCommand extends CommandWithSubcommands {
  private env: Env;
  name = "links";
  description = "Short-link root command";
  defer = true;
  subcommands: Command[];

  constructor(env: Env) {
    super();
    this.env = env;
    this.subcommands = [new CreateLink(this.env), new DeleteLink(this.env)];
  }
}
