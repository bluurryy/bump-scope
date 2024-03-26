inspect_asm::alloc_iter_u32::bumpalo:
	push r14
	push rbx
	push rax
	mov rax, rdx
	shr rax, 61
	jne .LBB_14
	lea rcx, [4*rdx]
	mov r8, qword ptr [rdi + 16]
	mov rax, qword ptr [r8 + 32]
	cmp rcx, rax
	ja .LBB_4
	sub rax, rcx
	and rax, -4
	cmp rax, qword ptr [r8]
	jb .LBB_4
	mov qword ptr [r8 + 32], rax
.LBB_5:
	test rdx, rdx
	je .LBB_16
	lea r8, [rdx - 1]
	cmp rdx, r8
	cmovb r8, rdx
	xor ecx, ecx
	cmp r8, 8
	jb .LBB_7
	mov r9, rax
	sub r9, rsi
	mov rdi, rsi
	cmp r9, 31
	jbe .LBB_9
	inc r8
	mov ecx, r8d
	and ecx, 7
	mov edi, 8
	cmovne rdi, rcx
	mov rcx, r8
	sub rcx, rdi
	lea rdi, [rsi + 4*rcx]
	xor r8d, r8d
.LBB_13:
	movups xmm0, xmmword ptr [rsi + 4*r8]
	movups xmm1, xmmword ptr [rsi + 4*r8 + 16]
	movups xmmword ptr [rax + 4*r8], xmm0
	movups xmmword ptr [rax + 4*r8 + 16], xmm1
	add r8, 8
	cmp rcx, r8
	jne .LBB_13
	jmp .LBB_9
.LBB_7:
	mov rdi, rsi
.LBB_9:
	lea rsi, [rsi + 4*rdx]
.LBB_10:
	cmp rdi, rsi
	je .LBB_11
	mov r8d, dword ptr [rdi]
	add rdi, 4
	mov dword ptr [rax + 4*rcx], r8d
	inc rcx
	cmp rdx, rcx
	jne .LBB_10
.LBB_16:
	add rsp, 8
	pop rbx
	pop r14
	ret
.LBB_4:
	mov rbx, rsi
	mov esi, 4
	mov r14, rdx
	mov rdx, rcx
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov rsi, rbx
	mov rdx, r14
	test rax, rax
	jne .LBB_5
.LBB_14:
	call qword ptr [rip + bumpalo::oom@GOTPCREL]
.LBB_11:
	lea rdi, [rip + .L__unnamed_0]
	lea rdx, [rip + .L__unnamed_1]
	mov esi, 34
	call qword ptr [rip + core::option::expect_failed@GOTPCREL]