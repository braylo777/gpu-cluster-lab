# Deploys

| When (CT) | Worker version | Deployment | Rollback point |
| --- | --- | --- | --- |
| 2026-10-28 09:45 | fffac5bf-a4b1-4394-942a-e61429f52751 | 4d4ca847-a503-43ad-bb1f-56b9d99da31d | First deploy. To take it offline without deleting, disable the workers.dev route; later deploys roll back to this version id (`wrangler rollback` or Dashboard → Deployments). |

Post-deploy check (09:45 CT): `/` returned 200.
