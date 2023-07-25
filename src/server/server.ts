import dotenv from "dotenv";
dotenv.config();

import express from "express";
import compression from "compression";
import fetch from "node-fetch";

import { uptimeRobotResponse } from "./uptimerobot";
import LastFM from "./music/lastfm.js";

const app: express.Application = express();
const port = 3000;

app.use(compression());

app.use(express.static("dist/page"));

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

const lastfm = new LastFM();
lastfm.init();
app.get("/api/lastfm", (req, res) => {
  res.json(lastfm.Tops);
});

app.listen(port, () => {
  console.log(`server started at http://localhost:${port}`);
});

