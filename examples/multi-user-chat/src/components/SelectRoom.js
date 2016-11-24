import React, { Component } from 'react';
import classnames from 'classnames';


export default class Room extends Component {

  title() {
    const { params: {roomName} } = this.props;
    return <h1>{ roomName }</h1>
  }
  render() {
    const { className } = this.props;
    return <div className={ classnames(className) }>
             MESSAGES
           </div>
  }
}
