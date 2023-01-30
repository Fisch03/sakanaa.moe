window.addEventListener('load',() => {
  fetch('/api/uptime')
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
      document.getElementById('UptimeText').innerText = `${avgUptime.toFixed(1)}%`;

      if(up == data.monitors.length) {
        document.getElementById('UptimeCurrent').innerText = 'everything works right now!';
      } else if(up == data.monitors.length - 1) {
        document.getElementById('UptimeCurrent').innerText = '1 service is down, uh oh!';
      } else {
        document.getElementById('UptimeCurrent').innerText = `${data.monitors.length - up} services are down, uh oh!`;
      }
    })
})