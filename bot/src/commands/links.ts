import {
  ApplicationCommandOptionType,
  Command,
  type CommandInteraction,
  type CommandOptions,
  CommandWithSubcommands,
} from "@buape/carbon";

class CreateLink extends Command {
  name = "create";
  description = "Create a short-link via Buape Link";
  options: CommandOptions = [
    {
      name: "domain",
      type: ApplicationCommandOptionType.String,
      description:
        "The domain this short-link should be created on (e.g. 'go.buape.com', 'go.kiai.app')",
      required: true,
      choices: [
        {
          name: "go.buape.com",
          value: "go.buape.com",
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
    const url = interaction.options?.getString("url");
    const slug = interaction.options?.getString("slug");

    console.log(interaction);

    if (!domain || !url || !slug) {
      return interaction.reply({ content: "Missing required options" });
    }

    try {
      const response = await fetch(`https://${domain}/create`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ url, slug }),
      });
      return interaction.reply({
        content: `Short-link created: ${response.url}`,
      });
    } catch (e) {
      return interaction.reply({ content: "Failed to create short-link" });
    }
  }
}

class Sub2 extends Command {
  name = "sub2";
  description = "Subcommand 2";
  defer = true;

  async run(interaction: CommandInteraction) {
    interaction.reply({ content: "Subcommand 2" });
  }
}

export default class Subc extends CommandWithSubcommands {
  name = "links";
  description = "Short-link root command";
  defer = true;

  subcommands = [new CreateLink(), new Sub2()];
}
