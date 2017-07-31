import React, { Component } from 'react'
import { Link } from 'react-router'
import classnames from 'classnames'
import * as server from '../server'

import './chat.css'

export default class Chat extends Component {

  render() {
    const { className, title, children } = this.props;
    return (
      <div className={classnames('Chat', className)}>
        <div className="room-list">
          <ul className="room-list--list">
          {
            server.room_list.map(room => (
              <li key={room.name} className="room-list--room">
                <Link className="room-list--room-name"
                  to={"/"+room.name}
                  >{ room.name }</Link>
                <span className="room-list--unread"
                  >{ room.unseen }</span>
              </li>
            ))
          }
          </ul>
        </div>
        <div className="chat-body">
          <div className="title-block">
            { title || <h1>No room selected</h1> }
           <span className="connection-status">[{ server.state }]</span>
          </div>
          <div className="message-field">
            { children }
          </div>
        </div>
      </div>
    );
  }
}
