import dotenv from "dotenv";
dotenv.config();

import express from "express";
import compression from "compression";
import fetch from "node-fetch";

import { uptimeRobotResponse } from "./uptimerobot";
import { MisskeyEmojis } from "./misskey";
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

let misskeyNotes: any;
let misskeyEmojis: any;
app.get("/api/misskey", (req, res) => {
  res.json({
    notes: misskeyNotes,
    emojis: misskeyEmojis,
    instance: process.env.MISSKEY_INSTANCE,
  });
});
async function fetchMisskey() {
  let res = await fetch(`https://${process.env.MISSKEY_INSTANCE}/api/users/notes`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({
      userId: process.env.MISSKEY_USERID,
      includeMyRenotes: false,
      includeReplies: false,
      excludeNsfw: true,
    })
  })
  if(res.status !== 200) throw new Error(res.statusText);
  misskeyNotes = await res.json();

  res = await fetch(`https://${process.env.MISSKEY_INSTANCE}/api/emojis`, {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({})
  })
  
  if(res.status !== 200) throw new Error(res.statusText);
  let data = await res.json() as MisskeyEmojis;

  let emojis: any = {};
  data.emojis.forEach((emoji: any) => {
    emojis[emoji.name] = emoji.url;
  });
  misskeyEmojis = emojis;
}
fetchMisskey();
setInterval(fetchMisskey, 1000*60);


app.listen(port, () => {
  console.log(`server started at http://localhost:${port}`);
});

