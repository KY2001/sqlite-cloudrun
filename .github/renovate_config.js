module.exports = {
  $schema: 'https://docs.renovatebot.com/renovate-schema.json',
  extends: ['config:recommended'],
  autodiscover: true,
  autodiscoverFilter: ['KY2001/*'],
  // separateMajorMinor: false,
  recreateWhen: 'always',
  prHourlyLimit: 0,
  prConcurrentLimit: 0,
  allowedCommands: [".*"],
  packageRules: [
    {
      matchManagers: ["dockerfile"],
      matchDepNames: ["docker/dockerfile"],
      enabled: false
    }
  ],
};
