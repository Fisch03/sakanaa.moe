window.addEventListener('load',() => {
  fetch('https://api.uptimerobot.com/v2/getMonitors', {
    method: 'POST',
    headers: {'Content-Type': 'application/x-www-form-urlencoded'},
    body: "custom_uptime_ratios=30&format=json&api_key=ur621781-14a03681c1d364f66e6573a9"
  })
    .then(res => res.json())
    .then(data => {
      console.log(data);
      let avgUptime = 0;
      let up = 0;
      data.monitors.forEach(element => {
        avgUptime += parseFloat(element.custom_uptime_ratio);
        if(element.status == 2) {
          up++;
        }  
      });
      avgUptime = avgUptime / data.monitors.length;
      document.getElementById('UptimeText').innerText = `${avgUptime.toFixed(2)}%`;
      document.getElementById('UptimeRatio').innerText = `${up}/${data.monitors.length}`;
    })
})