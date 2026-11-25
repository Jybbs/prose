ENDPOINTS = {
    "/repos/{owner}/{repo}/actions/runs/{run_id}/attempts/{attempt}" : [read],
    "/repos/{owner}/{repo}/pulls/{number}/reviewers"                 : [
        read,
        edit,
    ],
    "/repos/{owner}/{repo}/issues/{number}/comments": [read, list, edit, drop],
}
