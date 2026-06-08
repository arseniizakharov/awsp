// Shared sample profile data used across all variants.
// Designed to feel like a real org: prod/staging/dev, multiple accounts/regions.

const PROFILES = [
  { name: "acme-prod-admin",      account: "682471093210", role: "AdministratorAccess", region: "us-east-1",      sso: "acme-corp",  env: "prod" },
  { name: "acme-prod-readonly",   account: "682471093210", role: "ReadOnlyAccess",      region: "us-east-1",      sso: "acme-corp",  env: "prod" },
  { name: "acme-prod-billing",    account: "682471093210", role: "BillingAccess",       region: "us-east-1",      sso: "acme-corp",  env: "prod" },
  { name: "acme-staging-admin",   account: "447091823641", role: "AdministratorAccess", region: "us-west-2",      sso: "acme-corp",  env: "staging" },
  { name: "acme-staging-dev",     account: "447091823641", role: "DeveloperAccess",     region: "us-west-2",      sso: "acme-corp",  env: "staging" },
  { name: "acme-dev-sandbox",     account: "910237842156", role: "PowerUserAccess",     region: "us-west-2",      sso: "acme-corp",  env: "dev" },
  { name: "acme-dev-data",        account: "910237842156", role: "DataEngineerAccess",  region: "eu-west-1",      sso: "acme-corp",  env: "dev" },
  { name: "personal-playground",  account: "201938475610", role: "AdministratorAccess", region: "eu-central-1",   sso: "personal",   env: "personal" },
  { name: "client-northwind",     account: "584012736092", role: "ConsultantAccess",    region: "ap-southeast-2", sso: "northwind",  env: "client" },
  { name: "client-globex-prod",   account: "739102648351", role: "ReadOnlyAccess",      region: "us-east-2",      sso: "globex",     env: "client" },
];

const CURRENT = "acme-staging-dev";

const ENV_COLOR = {
  prod:    "#e07b7b",
  staging: "#e8a04a",
  dev:     "#7aa7d6",
  personal:"#c792ea",
  client:  "#8ec07c",
};

window.PROFILES = PROFILES;
window.CURRENT = CURRENT;
window.ENV_COLOR = ENV_COLOR;
