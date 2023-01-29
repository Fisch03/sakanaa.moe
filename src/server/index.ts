import express from "express";
import compression from "compression";
import fs from "fs";

const app: express.Application = express();
const port = 3000;

app.use(compression());

// Check if in dev scenario, if yes only act as api, dont serve static files
if(fs.existsSync("./dist/page/index.html")) {
  app.use(express.static("./dist/page"));
} else {
  console.log("Running in dev mode");
}
  
app.listen(port, () => {
  console.log(`server started at http://localhost:${port}`);
});