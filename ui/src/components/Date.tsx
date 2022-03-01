import React from 'react';
import { Space, Typography } from 'antd';
import Moment from 'react-moment';

const { Text } = Typography;

type DateProps = {
	children: string | number | Date | moment.Moment,
};

export default function(props: DateProps) {
	return (
		<Space>
		    <Text>{props.children}</Text>
		    <Text type="secondary">
		        <Moment fromNow>{props.children}</Moment>
		    </Text>
		</Space>
	);
}
