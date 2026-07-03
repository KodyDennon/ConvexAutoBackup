import type { Role, SecretKind } from "./appState";

export const TOKEN_STORAGE_KEY = "convex-autobackup.token";
export const roles: Role[] = ["owner", "admin", "operator", "viewer"];
export const secretKinds: SecretKind[] = [
  "convex_deploy_key",
  "s3_credentials",
  "webhook_token",
  "encryption_key"
];
export const weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
