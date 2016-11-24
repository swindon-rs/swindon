import React, { Component } from 'react';
import classnames from 'classnames';

export class Title extends Component {
  render() {
    const { params: {roomName} } = this.props;
    return <h1 className="room-title">{ roomName }</h1>
  }
}

export class Room extends Component {
  render() {
    const { className } = this.props;
    return <div className={ classnames(className) }>
             MESSAGES
           </div>
  }
}
