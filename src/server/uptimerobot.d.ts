export interface uptimeRobotResponse {
  stat: string;
  pagination: {
    offset: number;
    limit: number;
    total: number;
  };
  monitors: uptimeRobotMonitor[];
}

export interface uptimeRobotMonitor {
  id: number;
  friendly_name: string;
  url: string;
  type: uptimeRobotMonitorType;
  port: string;
  interval: number;
  timeout: number;
  status: number;
  create_datetime: number;
  custom_uptime_ratio: string;
}

export const enum uptimeRobotMonitorType {
  HTTP = 1,
  Keyword = 2,
  Ping = 3,
  Port = 4,
  Heartbeat = 5,
}