# 基于cq-http的eve查价机器人模块

使用方式：
* jita 三钛 
>  返回当前吉他售卖/买的价格

* jita 三钛! 
>  返回当前吉他售卖/买的详细订单（买卖各前三条）

查询流程（以“三钛”关键词为例）：



1. 查询对应的物品id（不足3字用空格补全，即“三钛 ”）：https://esi.evepc.163.com/latest/search/?categories=inventory_type&datasource=serenity&language=zh&search=三钛%20&strict=false

    返回`{"inventory_type":[25595,34424,34]}`


* (普通查询时：)除去市场中没有的id，截取前10种id轮询：
https://esi.evepc.163.com/latest/markets/10000002/orders/?datasource=serenity&order_type=all&page=1&type_id=25595

    返回买卖各一的价格

* (订单查询时：)除去市场中没有的id，取第一个进行查询：

    返回买卖各前三条