import Database from 'better-sqlite3';

import MusicBrainz from './musicbrainz.js';

import { TrackData } from "./tracktypes.js";

type DBTrackData = TrackData | undefined;
interface QueueItem extends TrackData {
  resolve: (track: TrackData) => void;
  reject: (reason: any) => void;
}

export default class TrackDB {
  db = new Database('db/cache.db');
  apiQueue: QueueItem[] = [];

  constructor() {
    this.db.pragma('journal_mode = WAL');

    this.db.exec('CREATE TABLE IF NOT EXISTS tracks (id INTEGER PRIMARY KEY, mbid TEXT, name TEXT, artist TEXT, album TEXT, cover TEXT)');
  }

  async fillTrackData(trackData: TrackData) : Promise<TrackData> {
    let track: DBTrackData;

    if(trackData.internalId) track = this.getFromID(trackData.internalId);
    if(!track)               track = this.getFromName(trackData.name, trackData.artist);
    if(!track)               track = await this.fetchNew(trackData);

    return track;
  }

  async fetchNew(trackData: TrackData) : Promise<TrackData> {
    let queueItem = trackData as QueueItem;
    
    return new Promise((resolve, reject) => {
      queueItem.resolve = (track: TrackData) => {
        this.db.prepare('INSERT INTO tracks (mbid, name, artist, album, cover) VALUES (?, ?, ?, ?, ?)')
        .run(track.mbid, track.name, track.artist, track.album, track.cover);

        let dbTrack = this.getFromName(queueItem.name, queueItem.artist);

        if(!dbTrack) throw new Error('Failed to fetch track from database');
        resolve(dbTrack);
      };

      queueItem.reject = reject;

      this.addToQueue(queueItem);
    });
  }

  /* --- API Queue Functions --- */
  private addToQueue(queueItem: QueueItem) {
    this.apiQueue.push(queueItem);
    if(this.apiQueue.length == 1) this.processQueue();
  }

  private async processQueue() {
    let queueItem = this.apiQueue[0];
    if(!queueItem) return;

    console.log(`Processing queue | total: ${this.apiQueue.length} | current: ${queueItem.name} - ${queueItem.artist}`);

    let track = await MusicBrainz.searchTrack(queueItem.name, queueItem.artist);

    queueItem.resolve(track);

    this.apiQueue.shift();
    if(this.apiQueue.length > 0) setTimeout(() => this.processQueue(), 1200); // Timeout is 1 second min + some some safety margin
    else console.log('Finished processing queue!');
  }

  /* --- Database Functions --- */
  private getFromID(id: number) : DBTrackData {
    return this.db.prepare('SELECT * FROM tracks WHERE id = ?').get(id) as DBTrackData;
  }

  private getFromName(name: string, artist: string) : DBTrackData {
    return this.db.prepare('SELECT * FROM tracks WHERE name = ? AND artist = ?').get(name, artist) as DBTrackData;
  }
}