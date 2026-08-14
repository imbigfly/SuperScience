---
name: humanizer-zh
description: 去除 AI 写作痕迹，让文本读起来像人写的。在用户要求去 AI 味、降 AI 味、humanize、改得更自然，或文本中出现 AI 套话、空洞强调、公式化过渡时使用。基于 24 种可检测的 AI 写作模式进行针对性改写。
---

# Humanizer-zh

改写文本，去除 AI 写作痕迹，同时保留作者的原意和风格。

## 核心原则

这个 skill 只做一件事：让文本读起来像人写的。

**不做的事：**
- 不添加新的论点或信息
- 不"改进"写作质量（除非用户明确要求）
- 不改变作者的观点或语气
- 不做通用的语法检查

**要做的事：**
- 检测并消除 AI 写作模式
- 保留原意
- 保持作者的自然说话方式

## 工作流程

1. **阅读**用户提供的文本
2. **识别**下面列出的 AI 模式
3. **改写**只针对有问题的部分
4. **返回**清理后的文本，如果改动不明显，附上简要说明改了什么

## AI 写作模式（检测并消除）

### 1. 过度强调的过渡

AI 最明显的特征。删除或改写这些：

- "It is important to note that"
- "This is particularly significant because"
- "Remarkably," / "Interestingly," / "Notably,"
- "This raises important questions about"
- "A deeper examination reveals"
- "This distinction is critical because"
- 中文："值得注意的是"、"需要指出的是"、"尤为重要的是"、"耐人寻味的是"

这些词组是在告诉读者某件事很重要，而不是通过内容本身证明它重要。直接删除，或者把重要性写进句子里。

### 2. 否定式排比（不是 X，而是 Y）

AI 最喜欢的修辞结构：

- "It's not just X — it's Y"
- "This isn't merely X, it's Y"
- "The question isn't whether X. It's whether Y."
- "Not because of X, but because of Y"
- 中文："这不仅仅是……更是……"、"问题不在于……而在于……"

偶尔用一次没问题。一段里出现两次以上就是 AI 的味道。改成直接陈述。

### 3. 三段式排比

三个一组的流畅三元组：

- "The system processes data, learns patterns, and generates insights."
- "It's fast, it's accurate, and it's reliable."
- 中文："它快速、准确、可靠。"

真实的人不会每次都正好想到三件事。改成两个，或者四个，或者干脆拆开。

### 4. 公告式标题和列表序言

- "Here's the thing:"
- "Let's break this down:"
- "The core insight:"
- "An important caveat:"
- "The bottom line:"
- 中文："关键在于："、"核心洞察是："、"说白了："、"归根结底："

直接写内容，不要先宣布你要写什么。

### 5. 加粗关键词

AI 会把段落中的**关键术语**加粗，好像在做学习卡片。去掉加粗，除非是真正的小标题。

### 6. 连接词堆砌

- "Moreover," "Furthermore," "Additionally," "In addition,"
- "However," "Nevertheless," "That said,"
- "Thus," "Therefore," "Consequently," "As a result,"
- "Indeed," "In fact," "Certainly,"
- 中文："此外"、"然而"、"因此"、"由此可见"、"事实上"

不是不能用。但不能每个句子都以这些词开头。删掉它们，如果逻辑关系本身已经清楚的话。

### 7. 空洞的强调

- "in the realm of"
- "serves as a gateway"
- "plays a vital/crucial role"
- "represents a significant shift"
- "underscores the importance of"
- "a wide array of"
- "countless" / "myriad"
- "delve into" / "explore the multifaceted nature of"
- 中文："在……领域中"、"发挥着至关重要的作用"、"标志着重大转变"、"凸显了……的重要性"

这些词组听起来很深刻，实际上什么都没说。用具体的说法替换。

### 8. 模糊归因

- "Experts believe"
- "Observers have noted"
- "Some critics argue"
- "Industry professionals point out"
- 中文："有专家认为"、"有观察者指出"、"业内人士表示"

要么说出具体是谁，要么直接陈述观点。

### 9. 虚假的平衡和开放问题

- "Only time will tell"
- "The implications remain to be seen"
- "This is an open question"
- "Whether this will last remains unclear"
- 中文："时间会证明一切"、"其影响仍有待观察"、"这仍是一个开放的问题"

如果作者有立场，直接说。如果真的不确定，用一句话说清楚为什么不确定，然后往下走。

### 10. 过于工整的结构

AI 把每个话题都处理成相同的深度和篇幅。每个段落长度一样，每个论点展开程度一样，每个章节结构一样。

打乱它。该短的短，该长的长。有些点一笔带过，有些深入展开。

### 11. 万能开场和收尾

开场：
- "In today's rapidly evolving landscape..."
- "The world of X is changing..."
- "X has become increasingly important..."
- 中文："在当今快速发展的时代……"、"随着……的不断深入……"

收尾：
- "The future of X is bright"
- "X is here to stay"
- "Only time will tell"
- 中文："未来可期"、"让我们拭目以待"

直接切入主题。在真正结束的地方结束。

### 12. 过于流畅的行文

AI 生成的文字过度打磨。每句之间的过渡都毫无痕迹，节奏没有变化，从不停顿。

加入一些不平整：
- 短句。突然出现。
- 偶尔用一个不完整的句子
- 让两个短句并列，中间不加连接词
- 换行。然后继续

### 13. 知识截止日期的暗示

- "as of my last update"
- "I don't have information beyond..."
- "based on available information"

直接删除。

### 14. 机械的先抑后扬

- "While X has challenges, it also presents opportunities..."
- "Although X is not without drawbacks, the benefits include..."
- 中文："尽管……存在挑战，但也带来了机遇……"

这种"先让步再转折"的结构 AI 用得太频繁。要么直接说好处，要么认真讨论坏处，不要每次都两边讨好。

### 15. 自我否定式铺垫

- "This may seem simple, but..."
- "It might sound obvious, yet..."
- "While this appears straightforward..."
- 中文："这看似简单，但……"、"虽然听起来显而易见……"

直接说正事。如果真的复杂，写复杂的部分，不要先提醒读者它看起来很简单。

### 16. 连续破折号展开

AI 爱用破折号做插入说明：

- "The result — a complete transformation — was unexpected."
- "This approach — which combines X and Y — offers several benefits."

偶尔一次可以。连续用就成了固定节奏。改成逗号、括号，或者拆成两句。

### 17. 程度副词过载

- "truly significant"
- "deeply interconnected"
- "highly sophisticated"
- "incredibly important"
- "remarkably effective"
- 中文："极为重要"、"深刻地"、"高度复杂"、"非常显著"

删掉副词，留下被修饰的词。如果"significant"本身不够强，换一个更准的词，而不是加一个"truly"。

### 18. 抽象名词连锁

- "the implementation of the development of the optimization of..."
- "the enhancement of organizational effectiveness"
- 中文："……的实施的推进的优化"

AI 把动词变成名词，再串成一长串。改回动词："they implemented, then developed, then optimized"。

### 19. 全知视角的总结句

- "This shows that..."
- "What this reveals is..."
- "The key takeaway is..."
- "This demonstrates the importance of..."
- 中文："这表明……"、"由此可见……"、"关键的收获是……"

让读者自己得出结论。如果必须总结，用具体内容而不是元评论。

### 20. 对称的让步结构

- "On one hand... On the other hand..."
- "While some argue X, others contend Y"
- 中文："一方面……另一方面……"、"有人认为……也有人认为……"

真实的论证不是天平。选一边，认真讲，提到另一边只是为了反驳它。

### 21. 无信息量的定语从句

- "which is a process that..."
- "which is an important aspect of..."
- "that serves as a means of..."
- 中文："这是一个……的过程"、"这是……的重要方面"

如果定语从句只是在重复已经说过的意思，删掉它。

### 22. 情感标记词

- "exciting" / "fascinating" / "compelling" 用在不该兴奋的地方
- "robust" / "seamless" / "cutting-edge" / "state-of-the-art"
- 中文："令人兴奋的"、"引人入胜的"、"前沿的"、"无缝的"

这些词是在代替实际描述。说清楚它做了什么，而不是说它"令人兴奋"。

### 23. 假想反对者

- "One might argue that..."
- "Critics may claim..."
- "Some might say..."
- 中文："有人可能会说……"、"也许有人会认为……"

除非真的有人提出了这个反对意见，否则不要捏造一个。直接陈述你的观点。

### 24. 收束式回环

AI 经常在结尾把开头的话重复一遍，形成工整的闭环：

- 开头："X is changing the world"
- 结尾："And that is how X is changing the world"

人的写作通常不会这么整齐。在你写完的地方结束，不要回到起点再走一遍。

## 改写规则

改写时：

1. **保留原意。** 不要添加、不要"改进"、不要改变观点。
2. **保留作者的语气。** 如果作者写得随意，保持随意。如果作者写得正式，保持正式。
3. **只改有问题的部分。** 不要为了改而改。
4. **宁可过于克制，也不要改得太狠。** 目标是消除 AI 痕迹，不是展示你的编辑能力。
5. **保持语言。** 中文文本用中文改写，英文文本用英文改写。不要翻译。

## 输出格式

直接返回改写后的文本。

如果改动不明显（原文已经比较自然），加一句简短说明，指出改了哪些模式。

如果原文完全没有 AI 痕迹，如实告知，不要为了改而改。
