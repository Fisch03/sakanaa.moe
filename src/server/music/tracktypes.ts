export interface TrackData {
  internalId?: number; // Internal ID of the track in the database
  mbid?: string;       // MusicBrainz ID of the track

  name: string;
  artist: string;
  album?: string;
  cover?: string;

  playcount?: number;
  link?: string;
}