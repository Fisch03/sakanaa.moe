export enum PlayableType {
  Track = 'track',
  Album = 'album',
  Artist = 'artist'
}

export abstract class Playable {
  type?: PlayableType;

  id?: number; // Internal ID in the database
  mbid?: string;   

  name: string;

  playcount?: number;

  constructor(name: string) {
    this.name = name;
  }
}

export class TrackData extends Playable {
  type = PlayableType.Track;

  artistId?: number;

  albumId?: number;
  link?: string;

  artistHelperField?: string;

  constructor(name: string, artistName?: string, artistId?: number) {
    super(name);
    this.artistHelperField = artistName;
    this.artistId = artistId;
  }
}

export class AlbumData extends Playable {
  type = PlayableType.Album;

  artistId?: number;

  cover?: string;

  artistHelperField?: string;

  constructor(name: string, artistName?: string, artistId?: number) {
    super(name);
    this.artistHelperField = artistName;
    this.artistId = artistId;
  }
}

export class ArtistData extends Playable {
  type = PlayableType.Artist;

  image?: string;

  constructor(name: string) {
    super(name);
  }
}
