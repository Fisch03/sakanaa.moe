import Database from 'better-sqlite3';

import MusicBrainz from './musicbrainz.js';

import { Playable, PlayableType, TrackData, AlbumData } from "./tracktypes.js";

type DBPlayable <T extends Playable> = T | undefined;

interface QueueItem<T extends Playable> extends Playable {
  retryCount?: number;
  resolve?: (result: T) => void;
  reject?: (reason: any) => void;
}

export default class TrackDB {
  db = new Database('db/cache.db');
  apiQueue: QueueItem<any>[] = [];

  constructor() {
    this.db.pragma('journal_mode = WAL');

    this.db.exec(`CREATE TABLE IF NOT EXISTS ${PlayableType.Album}s (
      id INTEGER PRIMARY KEY,
      mbid TEXT,

      name TEXT,
      artist TEXT,

      cover TEXT
    )`);

    this.db.exec(`CREATE TABLE IF NOT EXISTS ${PlayableType.Track}s (
      id INTEGER PRIMARY KEY, 
      mbid TEXT, 

      name TEXT, 
      artist TEXT,

      albumId INTEGER,
      FOREIGN KEY(albumId) REFERENCES ${PlayableType.Album}s(id)
    )`);
  }

  async fillData<T extends Playable>(playable: T) : Promise<T> {
    let res: DBPlayable<T>;

    res = this.fillFromDB<T>(playable);
    if(!res)        res = await this.fetchNew<T>(playable);

    if(!res) return playable;
    else     return res;
  }

  async fetchNew<T extends Playable>(playable: T) : Promise<T> {
    let queueItem = playable as QueueItem<T>;
    
    return new Promise((resolve, reject) => {
      queueItem.retryCount = 0;

      queueItem.resolve = (newItem: T) => {
        let rowid = this.insertPlayable<T>(newItem);

        let result = this.db.prepare(`SELECT * FROM ${newItem.type}s WHERE id = ?`).get(rowid) as T;
        resolve(result);
      };

      queueItem.reject = (reason: string) => {
        if(!queueItem.retryCount) queueItem.retryCount = 0;

        if(queueItem.retryCount < 3) {
          queueItem.retryCount++;
          console.log(`Failed to process ${queueItem.type}: ${reason} Retrying...`);
          this.addToQueue<T>(queueItem);
          return;
        } else {
          reject(`Failed to insert ${queueItem.type} into database: ${reason}`);
        }
      }

      this.addToQueue<T>(queueItem);
    });
  }

  /* --- API Queue Functions --- */
  private addToQueue<T extends Playable>(queueItem: QueueItem<T>) {
    this.apiQueue.push(queueItem);
    if(this.apiQueue.length == 1) this.processQueue();
  }

  private async processQueue() {
    let queueItem = this.apiQueue[0];
    if(!queueItem) return;

    console.log(`Processing queue | total: ${this.apiQueue.length} | current: ${queueItem.type} - ${queueItem.name}`);

    let res: Playable | undefined;
    switch(queueItem.type) {
      case PlayableType.Track:
        let track = queueItem as TrackData;
        res = await MusicBrainz.searchTrack(track, (album: AlbumData) => {
          if(album.mbid) {
            let albumByMBID = this.getAlbumFromMBID(album.mbid);
            if(albumByMBID) return albumByMBID.id;
          }

          console.log(`    Inserting new album "${album.name}" into database...`);

          return Number(this.insertPlayable<AlbumData>(album));
        });
        break;
    }

    if(!res) {
      if(queueItem.reject) queueItem.reject(`Failed to fetch info from API`);
    } 
    else {
      if(queueItem.resolve) queueItem.resolve(res);
    }

    this.apiQueue.shift();
    if(this.apiQueue.length > 0) setTimeout(() => this.processQueue(), 1200); // Timeout is 1 second min + some some safety margin
    else console.log('Finished processing queue!');
  }

  /* --- Database Functions --- */
  private insertPlayable<T extends Playable>(playable: T) : number | bigint {
    switch(playable.type) {
      case 'track':
        let track = playable as unknown as TrackData;
        return this.db.prepare('INSERT INTO tracks (mbid, name, artist, albumId) VALUES (?, ?, ?, ?)')
               .run(track.mbid, track.name, track.artist, track.albumId).lastInsertRowid;
      case 'album':
        let album = playable as unknown as AlbumData;
        return this.db.prepare('INSERT INTO albums (mbid, name, artist, cover) VALUES (?, ?, ?, ?)')
                .run(album.mbid, album.name, album.artist, album.cover).lastInsertRowid;

      default:
        throw new Error('Playable type not set');
    }
  }

  private fillFromDB<T extends Playable>(playable: T) : DBPlayable<T> {
    let res: DBPlayable<T>;

    if(playable.id) res = this.fillFromID(playable);
    if(!res)        res = this.fillFromName(playable);

    
    if(res) {
      res = {
        ...playable,
        ...res,
      }
    }
    return res;
  }

  private fillFromID<T extends Playable>(playable: T) : DBPlayable<T> {
    if(!playable.type) throw new Error('Playable type not set');

    return this.db.prepare(`SELECT * FROM ${playable.type}s WHERE id = ?`).get(playable.id) as DBPlayable<T>;
  }

  private fillFromName<T extends Playable>(playable: T) : DBPlayable<T> {
    if(!playable.type) throw new Error('Playable type not set');

    if('artist' in playable && playable.artist)
      return this.db.prepare(`SELECT * FROM ${playable.type}s WHERE name = ? AND artist = ?`).get(playable.name, playable.artist) as DBPlayable<T>;
    else
      return this.db.prepare(`SELECT * FROM ${playable.type}s WHERE name = ?`).get(playable.name) as DBPlayable<T>;
  }

  getAlbumFromID(id: number) : DBPlayable<AlbumData> {
    return this.db.prepare('SELECT * FROM albums WHERE id = ?').get(id) as DBPlayable<AlbumData>;
  }
  getAlbumFromMBID(mbid: string) : DBPlayable<AlbumData> {
    return this.db.prepare('SELECT * FROM albums WHERE mbid = ?').get(mbid) as DBPlayable<AlbumData>;
  }

}