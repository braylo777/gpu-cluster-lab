// Packs web/index.html + sim-report.json into one Worker module (dist/worker.js). Pure Node, no dependencies.
// Deploy: wrangler deploy dist/worker.js --name gpu-cluster-lab --compatibility-date 2026-10-01 --no-bundle
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
mkdirSync("dist", { recursive: true });
const H = readFileSync("web/index.html", "utf8"), R = readFileSync("sim-report.json", "utf8");
writeFileSync("dist/worker.js", `const H=${JSON.stringify(H)};const R=${JSON.stringify(R)};
export default{async fetch(q){const p=new URL(q.url).pathname;
if(p==="/")return new Response(H,{headers:{"content-type":"text/html; charset=utf-8"}});
if(p==="/sim-report.json")return new Response(R,{headers:{"content-type":"application/json"}});
return new Response("not found",{status:404})}};`);
console.log("built dist/worker.js");
