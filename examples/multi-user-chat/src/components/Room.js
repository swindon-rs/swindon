import React, { Component } from 'react';
import classnames from 'classnames';

import './room.css';

export default class Room extends Component {

  render() {
    const { className, params: {roomName} } = this.props;
    let room = roomName || 'general';
    return (
      <div className={classnames('Room', className)}>
        <div className="roster">
          ROSTER
        </div>
        <div className="chat-body">
          <h1 className="room-title">
            {room}
          </h1>
          <div className="messages">
              MESSAGES
          </div>
        </div>
      </div>
    );
  }
}
