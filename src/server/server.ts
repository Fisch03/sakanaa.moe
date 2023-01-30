import express from "express";
import compression from "compression";
import fetch from "node-fetch";

import { uptimeRobotResponse } from "./uptimerobot";

const app: express.Application = express();
const port = 3000;

app.use(compression());

let uptimestats: uptimeRobotResponse;
app.get("/api/uptime", (req, res) => {
  res.json(uptimestats);
});

setInterval(() => {
  console.log("fetching new uptime stats")

  fetch('https://api.uptimerobot.com/v2/getMonitors', {
    method: 'POST',
    headers: {'Content-Type': 'application/x-www-form-urlencoded'},
    body: "custom_uptime_ratios=30&format=json&api_key=ur621781-14a03681c1d364f66e6573a9"
  })
  .then(response => {
    if(response.status !== 200) throw new Error(response.statusText);
    return response.json()
  })
  .then(data => uptimestats = data as uptimeRobotResponse)
}, 1000*60);

app.listen(port, () => {
  console.log(`server started at http://localhost:${port}`);
});

