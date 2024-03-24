// 0x20: 0x40 hues but way worse.

performance.now = (function() {
    return performance.now    ||
        performance.mozNow    ||
        performance.msNow     ||
        performance.oNow      ||
        performance.webkitNow ||
        function() {
            return new Date().getTime(); 
        };
})();

let running_beat = null;
function play_beat(audio, beat_events, callback) {
    const beat_info = {
        src: audio,
        currentEvent: -1,
        beat_events: beat_events,

        currentBPM: 0,
        currentBPMTime: 0,

        callback: callback
    }; 

    beat_info.src.on('play', () => {
        beat_info.src.seek(0);
        beat_info.currentEvent = -1;
    });
    running_beat = beat_info;
    beat_info.src.play();
}


function process_beats() {  
    if(!running_beat) {
        requestAnimationFrame(process_beats);
        return;
    }

    const position = running_beat.src.seek() * 1000;

    if(running_beat.nextBeatTime < position) {
        if(running_beat.callback) running_beat.callback("Beat");
        running_beat.nextBeatTime += 60 / running_beat.currentBPM * 1000;
    }

    if(running_beat.currentEvent+1 >= running_beat.beat_events.length) {
        requestAnimationFrame(process_beats);
        return;
    } 

    const next_event = running_beat.beat_events[running_beat.currentEvent + 1]; 
    if(position < next_event.time_ms) {
        requestAnimationFrame(process_beats);
        return;
    }

    if(next_event.event_type.BPM) {
        running_beat.currentBPM = next_event.event_type.BPM;
        running_beat.nextBeatTime = next_event.time_ms + 60 / running_beat.currentBPM * 1000;
        console.log("BPM: " + running_beat.currentBPM, next_event.time_ms);
    }

    running_beat.currentEvent++;

    const diff = next_event.time_ms - position;
    //console.log(diff)
 
    requestAnimationFrame(process_beats)
}
process_beats();

let target_y = 0;
let y = 0;
let reactive_elements = []; 
function animate() {
    let diff = target_y - y;
    y += diff * 0.5;
    target_y *= 0.80;
    reactive_elements.forEach((element) => {
        element.style.transform = `translateY(${y}%)`;
    });
    requestAnimationFrame(animate);
}
animate();

async function getCurrentTrack() {
    return new Promise(async (resolve, reject) => {
        await fetch('/api/0x20').then(response => response.json()).then(data => {
            const track_info = {
                src: null,
                beat_events: data.beat_info
            };

            let track = new Howl({
                src: [data.track_location],
                format: ['mp3'],
                preload: true 
            });

            track.once('load', () => {
                track_info.src = track;
                resolve(track_info);
            });
        });
    });
}

window.addEventListener('load', () => {
    reactive_elements = document.querySelectorAll('.music_reactive');

    Howler.volume(0.1);

    let count = 0;
    function beat_react(event_type) {
        switch(event_type) {
            case "Beat":
                target_y += (count % 2 != 0) ? 2 : 4;

                /* if(count % 4 == 0) {
                    pointer = Math.random() * palettes.length | 0;

                    fgcolor = palettes[pointer].fg;
                    bgcolor = palettes[pointer].bg;
                    console.log(palettes[pointer].n);

                    applyColors();
                } */

                count++;
                break;
        }
    }

    document.getElementById("0x20Btn").addEventListener('click', (e) => {
        getCurrentTrack().then((track_info) => {
            console.log(track_info);
            play_beat(track_info.src, track_info.beat_events, beat_react);
        });
    });
});
