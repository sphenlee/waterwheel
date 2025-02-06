import React from 'react';
import { Space, Typography } from 'antd';

import dayjs, { Dayjs } from 'dayjs';
import relativeTime from 'dayjs/plugin/relativeTime';

dayjs.extend(relativeTime);

const { Text } = Typography;

type DateProps = {
	children: string | number | Dayjs,
};

export default function(props: DateProps) {
	return (
		<Space>
		    <Text>{props.children.toString()}</Text>
		    <Text type="secondary">
		        {dayjs(props.children).fromNow()}
		    </Text>
		</Space>
	);
}
