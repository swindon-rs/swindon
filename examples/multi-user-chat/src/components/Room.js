import React, { Component } from 'react';
import classnames from 'classnames';


export default class Room extends Component {

  render_title() {
    return <h1>No room selected</h1>
  }
  render() {
    const { className } = this.props;
    return <div className={ classnames(className) }>
        <p>
          <span style={{ fontSize: '500%' }}>↑</span>
            Enter room name in address bar
        </p>
        <p>
          <span style={{ fontSize: '500%' }}>←</span>
            Or select room from your room list (if you've visited before)
        </p>
       </div>
  }
}
