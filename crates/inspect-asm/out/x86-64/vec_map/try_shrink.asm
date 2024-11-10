inspect_asm::vec_map::try_shrink:
	mov rax, rdi
	movabs rcx, 4611686018427387900
	mov rdx, qword ptr [rsi]
	mov rdi, qword ptr [rsi + 8]
	mov r8, qword ptr [rsi + 16]
	mov rsi, qword ptr [rsi + 24]
	test rdi, rdi
	jle .LBB0_3
	push r15
	push r14
	push rbx
	lea r9, [rdx + 8*rdi]
	lea r10, [rdx + 8]
	cmp r9, r10
	cmova r10, r9
	mov rbx, rdx
	not rbx
	add rbx, r10
	mov r10, rdx
	mov r11, rdx
	cmp rbx, 24
	jb .LBB0_1
	shr rbx, 3
	inc rbx
	mov r14, rbx
	and r14, rcx
	lea r10, [rdx + 4*r14]
	lea r11, [rdx + 8*r14]
	xor r15d, r15d
.LBB0_0:
	movdqu xmm0, xmmword ptr [rdx + 8*r15]
	movdqu xmm1, xmmword ptr [rdx + 8*r15 + 16]
	pshufd xmm0, xmm0, 232
	pshufd xmm1, xmm1, 232
	punpcklqdq xmm0, xmm1
	movdqu xmmword ptr [rdx + 4*r15], xmm0
	add r15, 4
	cmp r14, r15
	jne .LBB0_0
	cmp rbx, r14
	je .LBB0_2
.LBB0_1:
	mov ebx, dword ptr [r11]
	mov dword ptr [r10], ebx
	add r11, 8
	add r10, 4
	cmp r11, r9
	jb .LBB0_1
.LBB0_2:
	pop rbx
	pop r14
	pop r15
.LBB0_3:
	add r8, r8
	add rcx, 2
	and rcx, r8
	mov qword ptr [rax], rdx
	mov qword ptr [rax + 8], rdi
	mov qword ptr [rax + 16], rcx
	mov qword ptr [rax + 24], rsi
	ret
