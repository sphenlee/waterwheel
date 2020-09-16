import React from 'react';
import { Space, Typography } from 'antd';
import Moment from 'react-moment';

const { Text } = Typography;

export default function(props) {
	return (
		<Space>
		    <Text>{props.children}</Text>
		    <Text type="secondary">
		        <Moment fromNow>{props.children}</Moment>
		    </Text>
		</Space>
	);
}
