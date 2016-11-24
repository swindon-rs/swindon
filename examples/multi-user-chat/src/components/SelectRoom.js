import React, { Component } from 'react';
import { Link } from 'react-router';
import classnames from 'classnames';

export class Main extends Component {
  render() {
    const { className } = this.props;
    return <div className={ classnames(className) }>
        <p>
          <span className="big-arrow">↑</span>
          Enter room name in address bar or here:
          &nbsp;
          <div className="select-room-input-box">
            <input type="text" placeholder="Room Name"
                onChange={ e => this.setState({roomName: e.target.value}) } />
            {this.state && this.state.roomName &&
                <Link to={"/" + this.state.roomName}>
                    <button>Go</button>
                </Link>}
            <Link to="/kittens">/kittens</Link>
            <Link to="/cars">/cars</Link>
          </div>
        </p>
        <p>
          <span className="big-arrow">←</span>
          Or select room from your room list (if you've visited before)
        </p>
       </div>
  }
}
