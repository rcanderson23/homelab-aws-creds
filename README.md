# homelab-aws-creds

Uses the EKS container endpoint in combination with the Kubernetes `ServiceAccount` token to give
the client temporary AWS credentials. This works similarly to `kiam` where credentials are assumed
by one entity and passed along to requester.

## Agent

Runs as a `DaemonSet` and serves AWS credentials to pods running on the same node. The pods send their
token as an auth header which is verified via a `TokenReview` request. If valid, the agent will respond with
credentials to the corresponding role credentials.

## Webhook

Mutates pods to have `AWS_CONTAINER_CREDENTIALS_FULL_URI`, `AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE`, and aws region environment variables if the pod service account matches one in the mapping config. The TLS config should automatically reload on cert renewal.

## Mapping Config

Maps the `ServiceAccount` name and `Namespace` to an AWS Role. This role must be able to be assumed by the
crednetials that the agent is using for AWS access. This mapping is automatically reloaded on change in both
the webhook and agent.

Example mapping:
```yaml
mappings:
  - serviceAccount: test
    namespace: default
    awsRole: arn:aws:iam::123456789000:role/read-only
```
