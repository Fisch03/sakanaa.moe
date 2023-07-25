import fetch from 'node-fetch';

import TrackDB from './trackdb.js';
import { AlbumData, TrackData } from "./tracktypes.js";

interface Release {
  id: string;
  title: string;
  date: string;

  'release-group': {
    id: string;
    'primary-type': string;
    'secondary-types': string[];
  };
}

interface Recording {
  id: string;

  title: string;
  releases: Release[];
}

interface RecordingResponse { recordings: Recording[]; }
interface ReleaseResponse   { releases: Release[]; }

export default class MusicBrainz {
  static async searchTrack(track: TrackData, db: TrackDB, createAlbum: (album: AlbumData) => number | undefined) : Promise<TrackData | undefined>{
    let res: TrackData = new TrackData(track.name, track.artist);
    
    let recording: any;
    let found = false;

    if(track.mbid) {
      recording = await this.doRequest('recording', `${track.mbid}?inc=artists+releases+release-groups`) as Promise<Recording>;
      if(recording) found = true;
    }
    
    if(!found) {
      let query = `?query=recording:"${encodeURIComponent(track.name)}" AND artist:"${encodeURIComponent(track.artist)}"`;
      let response = await this.doRequest('recording', query) as RecordingResponse;

      // Try to find recordings when the artist name contains something like "feat. ..., & ... etc."
      let artistsSplit = track.artist.split(' ');
      if(response.recordings.length == 0 && artistsSplit.length > 1) {
        let artistLoose = artistsSplit[0];
        console.log(`    Trying to find recording with artist "${artistLoose}"...`);
        response = await this.doRequest('recording', `?query=recording:"${track.name}" AND artist:"${artistLoose}"`) as RecordingResponse;
      }
      if(response.recordings.length == 0 && artistsSplit.length > 1) {
        let artistLoose = artistsSplit[artistsSplit.length - 1];
        console.log(`    Trying to find recording with artist "${artistLoose}"...`);
        response = await this.doRequest('recording', `?query=recording:"${track.name}" AND artist:"${artistLoose}"`) as RecordingResponse;
      }

      recording = response.recordings[0];

      // If there are multiple recordings try to find the most relevant one
      if(response.recordings.length > 1) {
        let recordings = response.recordings.filter(recording => {
          // Sort releases by date
          recording.releases.sort((a, b) => {
            if(a.date === b.date) return 0;
            if(a.date === null) return 1;
            if(b.date === null) return -1;
            return a.date < b.date ? -1 : 1;
          });

          // Filter out compilations and check if there is something left
          let recordingNoComp = recording.releases.find(release => {
            if(release['release-group']['primary-type'] === 'Compilation') return;
            if(release['release-group']['primary-type'] === 'Broadcast') return;
            if(release['release-group']['secondary-types'] && release['release-group']['secondary-types'].includes('Compilation')) return;

            return true;
          });

          if(recordingNoComp) recording.releases = [recordingNoComp];
          return recordingNoComp;
        });

        if(recordings.length > 0) {
          recording = recordings[0];
        }
      }
    }

    if(!recording) {
      console.log(`    No recording found...`);
      return res;
    }

    res.mbid = recording.id;
    
    if(!recording.releases || recording.releases.length == 0) {
      console.log(`    No release found...`);
      return res;
    }

    let release = recording.releases[0];
    recording.releases.forEach((testrelease: any) => {
      if(db.getAlbumFromMBID(testrelease['release-group'].id)) {
        release = testrelease;
      }
    });

    if(track.albumId) res.albumId = track.albumId;
    else {
      let newAlbum = new AlbumData(release.title, track.artist);
      newAlbum.mbid = release['release-group'].id;

      if(await this.checkForCover('release-group', release['release-group'].id))
        newAlbum.cover = `https://coverartarchive.org/release-group/${release['release-group'].id}/front-250`;
      else if(await this.checkForCover('release', release.id))
        newAlbum.cover = `https://coverartarchive.org/release/${release.id}/front-250`;
      else {
        console.log(`    No cover found...`);
      }

      res.albumId = createAlbum(newAlbum);
    }

    return res;
  }

  static async searchAlbum(album: AlbumData) : Promise<AlbumData | undefined> {
    let res: AlbumData = new AlbumData(album.name, album.artist);

    let release: any;
    let found = false;
    if(album.mbid) {
      release = await this.doRequest('release', `${album.mbid}?inc=artists+release-groups`) as Promise<Recording>;
      if(release) found = true;
    }

    if(!found) {
      let query = `?query=release:"${encodeURIComponent(album.name)}" AND artist:"${encodeURIComponent(album.artist)}"`;
      let response = await this.doRequest('release', query) as ReleaseResponse;

      release = response.releases[0];
    }

    if(!release) {
      console.log(`    No release found...`);
      return res;
    }

    res.mbid = release['release-group'].id;
    if(await this.checkForCover('release', release.id))
      res.cover = `https://coverartarchive.org/release-group/${release['release-group'].id}/front-250`;
    else {
      console.log(`    No cover found...`);
    }

    return res;	 
  }

  private static async doRequest(type: string, query: string): Promise<any>{
    //console.log(`    Fetching ${type} "${query}"...`)
    let response: any = await fetch(`https://musicbrainz.org/ws/2/${type}/${query}&fmt=json`, {
      headers: {
        'User-Agent': `sakanaa.moe/${process.env.npm_package_version} (${process.env.CONTACT_EMAIL})`
      },
    });
    if(response.status == 404) return undefined;
    if(response.status !== 200) throw new Error(response.statusText);
    try {
      response = response.json();
    } catch(e) {
      console.log(`    Failed to parse response...`);
      return undefined;
    }
    return response
  }

  private static async checkForCover(type: string, id: string) : Promise<boolean> {
    let response = await fetch(`https://coverartarchive.org/${type}/${id}`, {
      headers: {
        'User-Agent': `sakanaa.moe/${process.env.npm_package_version} (${process.env.CONTACT_EMAIL})`
      },
    });
    if(response.status == 404) return false;
    if(response.status !== 200) throw new Error(response.statusText);
    return true;
  }
}