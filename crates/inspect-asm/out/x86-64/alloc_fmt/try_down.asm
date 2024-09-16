inspect_asm::alloc_fmt::try_down:
	push r15
	push r14
	push r12
	push rbx
	sub rsp, 120
	mov qword ptr [rsp + 40], rsi
	mov qword ptr [rsp + 48], rdx
	lea rax, [rsp + 40]
	mov qword ptr [rsp + 56], rax
	lea rax, [rip + <&T as core::fmt::Display>::fmt]
	mov qword ptr [rsp + 64], rax
	lea rax, [rip + .L__unnamed_0]
	mov qword ptr [rsp + 72], rax
	mov qword ptr [rsp + 80], 2
	mov qword ptr [rsp + 104], 0
	lea rax, [rsp + 56]
	mov qword ptr [rsp + 88], rax
	mov qword ptr [rsp + 96], 1
	movups xmm0, xmmword ptr [rip + .L__unnamed_1]
	movaps xmmword ptr [rsp], xmm0
	mov qword ptr [rsp + 16], 0
	mov qword ptr [rsp + 24], rdi
	lea rsi, [rip + .L__unnamed_2]
	mov rdi, rsp
	lea rdx, [rsp + 72]
	call qword ptr [rip + core::fmt::write@GOTPCREL]
	test al, al
	je .LBB0_1
	mov rax, qword ptr [rsp]
	mov rcx, qword ptr [rsp + 24]
	mov rcx, qword ptr [rcx]
	cmp rax, qword ptr [rcx]
	jne .LBB0_0
	add rax, qword ptr [rsp + 16]
	mov qword ptr [rcx], rax
.LBB0_0:
	xor eax, eax
	jmp .LBB0_9
.LBB0_1:
	mov rsi, qword ptr [rsp]
	mov rbx, qword ptr [rsp + 8]
	mov rax, qword ptr [rsp + 16]
	mov r15, qword ptr [rsp + 24]
	mov rcx, qword ptr [r15]
	mov rcx, qword ptr [rcx]
	cmp rsi, rcx
	je .LBB0_3
	mov r14, rsi
	cmp r14, rcx
	je .LBB0_6
.LBB0_2:
	mov rax, r14
	jmp .LBB0_9
.LBB0_3:
	add rax, rsi
	xor r14d, r14d
	sub rax, rbx
	cmovae r14, rax
	lea rax, [rbx + rsi]
	mov rdi, r14
	mov rdx, rbx
	cmp rax, r14
	jbe .LBB0_4
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB0_5
.LBB0_4:
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB0_5:
	mov rax, qword ptr [r15]
	mov qword ptr [rax], r14
	mov rax, qword ptr [r15]
	mov rcx, qword ptr [rax]
	mov rax, rbx
	cmp r14, rcx
	jne .LBB0_2
.LBB0_6:
	add rax, rcx
	xor edx, edx
	sub rax, rbx
	cmovae rdx, rax
	lea rax, [rbx + rcx]
	mov rdi, rdx
	sub rdi, rcx
	add rdi, r14
	mov r12, rdi
	mov rsi, r14
	cmp rax, rdx
	jbe .LBB0_7
	mov rdx, rbx
	call qword ptr [rip + memmove@GOTPCREL]
	jmp .LBB0_8
.LBB0_7:
	mov rdx, rbx
	call qword ptr [rip + memcpy@GOTPCREL]
.LBB0_8:
	mov rcx, qword ptr [r15]
	mov rax, r12
	mov qword ptr [rcx], r12
.LBB0_9:
	mov rdx, rbx
	add rsp, 120
	pop rbx
	pop r12
	pop r14
	pop r15
	ret
	mov rcx, qword ptr [rsp]
	mov rdx, qword ptr [rsp + 24]
	mov rdx, qword ptr [rdx]
	cmp rcx, qword ptr [rdx]
	jne .LBB0_10
	add rcx, qword ptr [rsp + 16]
	mov qword ptr [rdx], rcx
.LBB0_10:
	mov rdi, rax
	call _Unwind_Resume@PLT
