import React, { Component } from 'react';
import classnames from 'classnames';

import './chat.css';

export default class Chat extends Component {

  render() {
    const { className, children, params: {roomName} } = this.props;
    return (
      <div className={classnames('Chat', className)}>
        <div className="roster">
          ROSTER
        </div>
        <div className="chat-body">
          <h1 className="room-title">
            { React.Children.only(this.children).render_title() }
          </h1>
          <div className="messages">
            { this.children }
          </div>
        </div>
      </div>
    );
  }
}
