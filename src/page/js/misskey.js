window.addEventListener('load',fetchMisskey);

const mkElem = document.getElementById('MisskeyNotes');

const   mkEmojiRegex = /:\w+:/g;
const textEmojiRegex = /(?<!>)\p{Extended_Pictographic}/gu; //Match any emoji that is not in a html tag, not the most accurate but it works

function fetchMisskey() {
  fetch('/api/misskey')
  .then(res => res.json())
  .then(data => {
    data.notes.forEach(note => {
      const notecontainer = document.createElement('div');
      notecontainer.classList.add('mbpost', 'clickable');
      notecontainer.onclick = () => {
        window.open(`https://${data.instance}/notes/${note.id}`);
      };

      const noteheader = document.createElement('div');
      noteheader.classList.add('mbpostheader'); 
      noteheader.innerHTML= `
        <span>@${note.user.username}@${note.user.host || data.instance}</span>
        <span>${new Date(note.createdAt).toLocaleString()}</span>
      `
      notecontainer.appendChild(noteheader);

      /*
      const noteavatar = document.createElement('img');
      noteavatar.classList.add('mbpostavatar');
      noteavatar.src = note.user.avatarUrl;
      noteheader.appendChild(noteavatar);
      */

      const notecontent = document.createElement('div');
      notecontent.classList.add('mbpostcontent');
      notecontainer.appendChild(notecontent);
      
      let text = note.text;
      let match;
      while(match = mkEmojiRegex.exec(text)) {
        const emoji = match[0].replace(/:/g,'');
        text = text.replace(match[0],`<img class="mbemoji colorfilter" src="${data.emojis[emoji]}" />`);
      }
      while(match = textEmojiRegex.exec(text)) {
        const emoji = match[0];
        text = text.replace(match[0],`<span class=colorfilter>${emoji}</span>`);
      }
      notecontent.innerHTML = text;

      if(note.files.length > 0) {
        const noteimages = document.createElement('div');
        noteimages.classList.add('mbpostimages');
        notecontent.appendChild(noteimages);

        note.files.forEach(file => {
          const noteimage = document.createElement('img');
          noteimage.classList.add('mbpostimage', 'colorfilter');
          noteimage.src = file.url;
          noteimages.appendChild(noteimage);
        });
      }

      mkElem.appendChild(notecontainer);
    });
  })
}