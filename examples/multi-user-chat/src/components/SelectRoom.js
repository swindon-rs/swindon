import React, { Component } from 'react';
import { Link, browserHistory } from 'react-router';
import classnames from 'classnames';


export class Main extends Component {
  render() {
    const { className } = this.props;
    return <div className={ classnames(className) }>
        <p>
          <span className="big-arrow">↑</span>
          Enter room name in address bar or here:
          &nbsp;
          <span className="select-room-input-block">
            <span className="select-room-input-box">
            <input type="text" placeholder="Room Name"
                onChange={ e => this.setState({roomName: e.target.value}) }
                onKeyDown={ e => {
                    if(e.which === 13 && this.state && this.state.roomName) {
                        browserHistory.push('/' + this.state.roomName)
                    }
                }}/>
            {this.state && this.state.roomName &&
                <Link to={"/" + this.state.roomName}>
                    <button>Go</button>
                </Link>}
            </span>
            <Link to="/kittens">/kittens</Link>
            <Link to="/cars">/cars</Link>
          </span>
        </p>
        <p>
          <span className="big-arrow">←</span>
          Or select room from your room list (if you've visited before)
        </p>
       </div>
  }
}
